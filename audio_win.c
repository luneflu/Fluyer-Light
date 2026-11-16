/* audio_win.c - Media Foundation decodes, WASAPI shared mode plays.
   Same contract as audio.c: one decoder pulled straight from the render thread,
   no ring buffer, no worker queue. The client runs at the file's own rate and
   WASAPI resamples (AUTOCONVERTPCM), so MF only has to decode. Files loop. */

#define COBJMACROS
#define INITGUID // MMDevice/WASAPI interface ids without depending on uuid.lib contents
#define _WIN32_WINNT 0x0A00
#define WINVER 0x0A00
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <audiopolicy.h>
#include <stdio.h>
#include <string.h>

/* The SDK only declares these ids; uuid.lib no longer carries them, so define
   them here (DECLSPEC_SELECTANY, so they merge harmlessly). */
DEFINE_GUID(CLSID_MMDeviceEnumerator, 0xBCDE0395, 0xE52F, 0x467C, 0x8E, 0x3D, 0xC4, 0x57, 0x92,
            0x91, 0x69, 0x2E);
DEFINE_GUID(IID_IMMDeviceEnumerator, 0xA95664D2, 0x9614, 0x4F35, 0xA7, 0x46, 0xDE, 0x8D, 0xB6,
            0x36, 0x17, 0xE6);
DEFINE_GUID(IID_IAudioClient, 0x1CB9AD4C, 0xDBFA, 0x4C32, 0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03,
            0xB2);
DEFINE_GUID(IID_IAudioRenderClient, 0xF294ACFC, 0x3146, 0x4483, 0xA7, 0xBF, 0xAD, 0xDC, 0xA7, 0xC2,
            0x60, 0xE2);
DEFINE_GUID(IID_ISimpleAudioVolume, 0x87CE5498, 0x68D6, 0x44E5, 0x92, 0x15, 0x6D, 0xA4, 0x7E, 0xF8,
            0x83, 0xD8);

enum { REQ_NONE, REQ_PLAY, REQ_STOP, REQ_TOGGLE, REQ_SEEK, REQ_VOL, REQ_QUIT };

static IAudioClient *g_client;
static IAudioRenderClient *g_render;
static ISimpleAudioVolume *g_vol;
static IMFSourceReader *g_reader;
static UINT32 g_buf_frames, g_rate;
static HANDLE g_render_ev, g_cmd_ev, g_ack_ev;
static CRITICAL_SECTION g_lock;
static int g_playing, g_ok, g_dev_open;
static double g_len;
static LONG64 g_frames_done;
static char g_path[1024];

/* pending command, guarded by g_lock */
static int g_req;
static char g_req_path[1024];
static double g_req_seek;
static float g_req_vol;

/* one decoded chunk at a time; WASAPI asks for frame counts that never line up */
static BYTE *g_pend;
static DWORD g_pend_len, g_pend_off;

static void chunks_free(void) {
  if (g_pend) {
    CoTaskMemFree(g_pend);
    g_pend = NULL;
  }
  g_pend_len = g_pend_off = 0;
}

/* ---- decoder (render thread only) ---- */

static void reader_release(void) {
  if (g_reader) {
    IMFSourceReader_Release(g_reader);
    g_reader = NULL;
  }
  chunks_free();
  g_len = 0.0;
  InterlockedExchange64(&g_frames_done, 0);
  g_path[0] = 0;
}

static int reader_open(const char *path, UINT32 *rate) {
  wchar_t wpath[1024];
  if (!MultiByteToWideChar(CP_UTF8, 0, path, -1, wpath, 1024)) return 0;
  if (FAILED(MFCreateSourceReaderFromURL(wpath, NULL, &g_reader))) return 0;

  UINT32 src_rate = 44100, ch = 2;
  IMFMediaType *native = NULL;
  if (SUCCEEDED(IMFSourceReader_GetNativeMediaType(g_reader, MF_SOURCE_READER_FIRST_AUDIO_STREAM, 0,
                                                   &native))) {
    IMFMediaType_GetUINT32(native, &MF_MT_AUDIO_SAMPLES_PER_SECOND, &src_rate);
    IMFMediaType_Release(native);
  }

  IMFMediaType *out = NULL;
  MFCreateMediaType(&out);
  IMFMediaType_SetGUID(out, &MF_MT_MAJOR_TYPE, &MFMediaType_Audio);
  IMFMediaType_SetGUID(out, &MF_MT_SUBTYPE, &MFAudioFormat_PCM);
  IMFMediaType_SetUINT32(out, &MF_MT_AUDIO_NUM_CHANNELS, ch);
  IMFMediaType_SetUINT32(out, &MF_MT_AUDIO_BITS_PER_SAMPLE, 16);
  IMFMediaType_SetUINT32(out, &MF_MT_AUDIO_SAMPLES_PER_SECOND, src_rate);
  HRESULT hr =
      IMFSourceReader_SetCurrentMediaType(g_reader, MF_SOURCE_READER_FIRST_AUDIO_STREAM, NULL, out);
  IMFMediaType_Release(out);
  if (FAILED(hr)) {
    reader_release();
    return 0;
  }

  PROPVARIANT v;
  PropVariantInit(&v);
  if (SUCCEEDED(IMFSourceReader_GetPresentationAttribute(g_reader, MF_SOURCE_READER_MEDIASOURCE,
                                                         &MF_PD_DURATION, &v)) &&
      v.vt == VT_UI8)
    g_len = (double)v.uhVal.QuadPart / 1e7;
  PropVariantClear(&v);

  strncpy_s(g_path, sizeof g_path, path, sizeof g_path - 1);
  *rate = src_rate;
  return 1;
}

/* ---- device (render thread only) ---- */

static void device_close(void) {
  if (g_client) IAudioClient_Stop(g_client);
  if (g_vol) ISimpleAudioVolume_Release(g_vol);
  if (g_render) IAudioRenderClient_Release(g_render);
  if (g_client) IAudioClient_Release(g_client);
  g_vol = NULL;
  g_render = NULL;
  g_client = NULL;
  g_dev_open = 0;
}

static int device_open(UINT32 rate) {
  IMMDeviceEnumerator *en = NULL;
  IMMDevice *dev = NULL;
  HRESULT hr;
  CoCreateInstance(&CLSID_MMDeviceEnumerator, NULL, CLSCTX_ALL, &IID_IMMDeviceEnumerator,
                   (void **)&en);
  if (!en) return 0;
  hr = IMMDeviceEnumerator_GetDefaultAudioEndpoint(en, eRender, eConsole, &dev);
  IMMDeviceEnumerator_Release(en);
  if (FAILED(hr)) return 0;
  hr = IMMDevice_Activate(dev, &IID_IAudioClient, CLSCTX_ALL, NULL, (void **)&g_client);
  IMMDevice_Release(dev);
  if (FAILED(hr)) return 0;

  WAVEFORMATEX wf;
  memset(&wf, 0, sizeof wf);
  wf.wFormatTag = WAVE_FORMAT_PCM;
  wf.nChannels = 2;
  wf.nSamplesPerSec = rate;
  wf.wBitsPerSample = 16;
  wf.nBlockAlign = (WORD)(wf.nChannels * wf.wBitsPerSample / 8);
  wf.nAvgBytesPerSec = wf.nSamplesPerSec * wf.nBlockAlign;

  // AUTOCONVERTPCM: WASAPI accepts our rate and resamples to the mix format.
  hr = IAudioClient_Initialize(g_client, AUDCLNT_SHAREMODE_SHARED,
                               AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM |
                                   AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                               2000000, 0, &wf, NULL);
  if (FAILED(hr)) return 0;
  hr = IAudioClient_GetBufferSize(g_client, &g_buf_frames);
  if (FAILED(hr)) return 0;
  hr = IAudioClient_SetEventHandle(g_client, g_render_ev);
  if (FAILED(hr)) return 0;
  hr = IAudioClient_GetService(g_client, &IID_IAudioRenderClient, (void **)&g_render);
  if (FAILED(hr)) return 0;
  if (FAILED(IAudioClient_GetService(g_client, &IID_ISimpleAudioVolume, (void **)&g_vol)))
    g_vol = NULL;
  if (g_vol) ISimpleAudioVolume_SetMasterVolume(g_vol, g_req_vol > 0.0f ? g_req_vol : 1.0f, NULL);
  IAudioClient_Start(g_client);
  g_dev_open = 1;
  g_rate = rate;
  return 1;
}

/* ---- fill: pull decoded frames straight into the WASAPI buffer ---- */

static void fill(BYTE *out, UINT32 frames) {
  const UINT32 need = frames * 4; // PCM16 stereo
  UINT32 w = 0;
  while (w < need) {
    if (!g_playing || !g_reader) {
      memset(out + w, 0, need - w);
      return;
    }
    if (g_pend && g_pend_off < g_pend_len) {
      DWORD take = g_pend_len - g_pend_off;
      if (take > need - w) take = need - w;
      memcpy(out + w, g_pend + g_pend_off, take);
      g_pend_off += take;
      w += take;
      if (g_pend_off >= g_pend_len) chunks_free();
      continue;
    }

    DWORD flags = 0;
    IMFSample *sample = NULL;
    HRESULT hr = IMFSourceReader_ReadSample(g_reader, MF_SOURCE_READER_FIRST_AUDIO_STREAM, 0, NULL,
                                            &flags, NULL, &sample);
    if (FAILED(hr)) {
      memset(out + w, 0, need - w);
      return;
    }
    if (flags & MF_SOURCE_READERF_ENDOFSTREAM) {
      PROPVARIANT v;
      PropVariantInit(&v);
      v.vt = VT_I8;
      IMFSourceReader_SetCurrentPosition(g_reader, &GUID_NULL, &v);
      PropVariantClear(&v);
      InterlockedExchange64(&g_frames_done, 0);
      if (sample) IMFSample_Release(sample);
      continue; /* loop */
    }
    if (!sample) continue;

    IMFMediaBuffer *buf = NULL;
    if (FAILED(IMFSample_ConvertToContiguousBuffer(sample, &buf)) || !buf) {
      IMFSample_Release(sample);
      continue;
    }
    BYTE *data = NULL;
    DWORD len = 0;
    if (FAILED(IMFMediaBuffer_Lock(buf, &data, NULL, &len)) || !len) {
      IMFMediaBuffer_Unlock(buf);
      IMFMediaBuffer_Release(buf);
      IMFSample_Release(sample);
      continue;
    }
    g_pend = (BYTE *)CoTaskMemAlloc(len);
    if (g_pend) {
      memcpy(g_pend, data, len);
      g_pend_len = len;
      g_pend_off = 0;
    }
    IMFMediaBuffer_Unlock(buf);
    IMFMediaBuffer_Release(buf);
    IMFSample_Release(sample);
  }
}

/* ---- commands: the render thread owns every decoder + device object ---- */

static void do_play(const char *path) {
  reader_release();
  g_playing = 0;
  UINT32 rate = 0;
  if (!reader_open(path, &rate)) {
    g_ok = 0;
    return;
  }
  if (!g_dev_open || rate != g_rate) {
    device_close();
    if (!device_open(rate)) {
      reader_release();
      g_ok = 0;
      return;
    }
  }
  g_playing = 1;
  IAudioClient_Start(g_client);
  g_ok = 1;
}

static void do_seek(double seconds) {
  if (!g_reader || seconds < 0) {
    g_ok = 0;
    return;
  }
  PROPVARIANT v;
  PropVariantInit(&v);
  v.vt = VT_I8;
  v.hVal.QuadPart = (LONGLONG)(seconds * 1e7);
  HRESULT hr = IMFSourceReader_SetCurrentPosition(g_reader, &GUID_NULL, &v);
  PropVariantClear(&v);
  chunks_free();
  InterlockedExchange64(&g_frames_done, (LONG64)(seconds * g_rate));
  g_ok = SUCCEEDED(hr);
}

static void do_cmd(int req) {
  switch (req) {
  case REQ_PLAY:
    do_play(g_req_path);
    break;
  case REQ_STOP:
    g_playing = 0;
    reader_release();
    if (g_client) IAudioClient_Stop(g_client);
    g_ok = 1;
    break;
  case REQ_TOGGLE:
    if (!g_reader) {
      g_ok = 0;
      break;
    }
    g_playing = !g_playing;
    if (g_client) {
      if (g_playing) IAudioClient_Start(g_client);
      else IAudioClient_Stop(g_client);
    }
    g_ok = 1;
    break;
  case REQ_SEEK:
    do_seek(g_req_seek);
    break;
  case REQ_VOL:
    if (g_vol) ISimpleAudioVolume_SetMasterVolume(g_vol, g_req_vol, NULL);
    g_ok = 1;
    break;
  }
}

static DWORD WINAPI render_thread(LPVOID unused) {
  (void)unused;
  CoInitializeEx(NULL, COINIT_MULTITHREADED);
  HANDLE waits[2] = {g_render_ev, g_cmd_ev};

  for (;;) {
    DWORD r = WaitForMultipleObjects(2, waits, FALSE, g_dev_open ? 200 : INFINITE);
    int quitting = 0;
    EnterCriticalSection(&g_lock);
    int req = g_req;
    if (req) g_req = REQ_NONE;
    LeaveCriticalSection(&g_lock);

    if (req == REQ_QUIT) {
      g_ok = 1;
      SetEvent(g_ack_ev);
      quitting = 1;
    } else if (req) {
      do_cmd(req);
      SetEvent(g_ack_ev);
    }
    if (quitting) break;

    if (r == WAIT_OBJECT_0 && g_dev_open) {
      UINT32 pad = 0;
      if (FAILED(IAudioClient_GetCurrentPadding(g_client, &pad))) continue;
      UINT32 n = g_buf_frames - pad;
      if (!n) continue;
      BYTE *out = NULL;
      if (FAILED(IAudioRenderClient_GetBuffer(g_render, n, &out)) || !out) continue;
      EnterCriticalSection(&g_lock);
      fill(out, n);
      InterlockedExchangeAdd64(&g_frames_done, n);
      LeaveCriticalSection(&g_lock);
      IAudioRenderClient_ReleaseBuffer(g_render, n, 0);
    }
  }

  reader_release();
  device_close();
  CoUninitialize();
  return 0;
}

/* ---- api ---- */

static int send(int req) {
  EnterCriticalSection(&g_lock);
  g_req = req;
  LeaveCriticalSection(&g_lock);
  SetEvent(g_cmd_ev);
  if (WaitForSingleObject(g_ack_ev, 5000) != WAIT_OBJECT_0) return 0;
  return g_ok;
}

int au_init(void) {
  InitializeCriticalSection(&g_lock);
  CoInitializeEx(NULL, COINIT_MULTITHREADED);
  MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);
  g_render_ev = CreateEventA(NULL, FALSE, FALSE, NULL);
  g_cmd_ev = CreateEventA(NULL, FALSE, FALSE, NULL);
  g_ack_ev = CreateEventA(NULL, FALSE, FALSE, NULL);
  g_req_vol = 1.0f;
  HANDLE t = CreateThread(NULL, 0, render_thread, NULL, 0, NULL);
  if (!t) return 0;
  CloseHandle(t);
  return 1; // device opens on first play, so idle costs nothing
}

int au_play(const char *path) {
  EnterCriticalSection(&g_lock);
  strncpy_s(g_req_path, sizeof g_req_path, path, sizeof g_req_path - 1);
  LeaveCriticalSection(&g_lock);
  return send(REQ_PLAY);
}

void au_stop(void) { send(REQ_STOP); }

int au_toggle(void) { return send(REQ_TOGGLE); }

int au_playing(void) { return g_playing && g_reader != NULL; }

double au_pos(void) {
  UINT32 rate = g_rate;
  if (!g_reader || !rate) return 0.0;
  return (double)InterlockedCompareExchange64(&g_frames_done, 0, 0) / (double)rate;
}

double au_len(void) { return g_len; }

int au_seek(double seconds) {
  g_req_seek = seconds;
  return send(REQ_SEEK);
}

void au_set_volume(float v) {
  g_req_vol = v;
  send(REQ_VOL);
}

const char *au_path(void) { return g_path; }

void au_quit(void) {
  send(REQ_QUIT);
  MFShutdown();
  CoUninitialize();
}
