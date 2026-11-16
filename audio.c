/* audio: CoreAudio/AudioToolbox only. ExtAudioFile decodes, default-output AUHAL plays.
   No miniaudio: AudioFile already decodes flac/mp3/aac/alac/wav from the OS. */

#include <AudioToolbox/AudioToolbox.h>
#include <AudioUnit/AudioUnit.h>
#include <string.h>

static ExtAudioFileRef g_file;
static AudioUnit g_au;
static int g_au_ready, g_loaded, g_playing;
static char g_path[1024];

/* Real-time thread: pull straight from the decoder, no ring buffer, no worker threads. */
static OSStatus render(void *in, AudioUnitRenderActionFlags *fl, const AudioTimeStamp *ts,
                       UInt32 bus, UInt32 frames, AudioBufferList *io) {
  (void)in; (void)fl; (void)ts; (void)bus;
  if (!g_file) {
    for (UInt32 b = 0; b < io->mNumberBuffers; b++)
      memset(io->mBuffers[b].mData, 0, io->mBuffers[b].mDataByteSize);
    return noErr;
  }
  UInt32 n = frames;
  OSStatus e = ExtAudioFileRead(g_file, &n, io);
  if (e != noErr) n = 0;
  if (n < frames) {
    for (UInt32 b = 0; b < io->mNumberBuffers; b++) {
      UInt32 left = frames - n;
      memset((char *)io->mBuffers[b].mData + n * io->mBuffers[b].mNumberChannels * 2, 0,
             left * io->mBuffers[b].mNumberChannels * 2);
    }
    ExtAudioFileSeek(g_file, 0); /* loop */
  }
  return noErr;
}

int au_init(void) { return 1; } /* device opens on first play, so idle costs nothing */

static int open_device(UInt32 rate) {
  AudioStreamBasicDescription out = {0};
  out.mSampleRate = rate;
  out.mFormatID = kAudioFormatLinearPCM;
  out.mFormatFlags = kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked;
  out.mChannelsPerFrame = 2;
  out.mBitsPerChannel = 16;
  out.mBytesPerPacket = 4;
  out.mFramesPerPacket = 1;
  out.mBytesPerFrame = 4;

  AudioComponentDescription d = {kAudioUnitType_Output, kAudioUnitSubType_DefaultOutput,
                                 kAudioUnitManufacturer_Apple, 0, 0};
  AudioComponent comp = AudioComponentFindNext(NULL, &d);
  if (!comp) return 0;
  if (AudioComponentInstanceNew(comp, &g_au) != noErr) return 0;
  AURenderCallbackStruct rcs = {render, NULL};
  if (AudioUnitSetProperty(g_au, kAudioUnitProperty_SetRenderCallback, kAudioUnitScope_Input, 0,
                           &rcs, sizeof rcs) != noErr) return 0;
  if (AudioUnitSetProperty(g_au, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Input, 0, &out,
                           sizeof out) != noErr) return 0;
  if (AudioUnitInitialize(g_au) != noErr) return 0;
  g_au_ready = 1;
  return 1;
}

static void unload(void) {
  if (g_au_ready) AudioOutputUnitStop(g_au);
  g_playing = 0;
  if (g_loaded) {
    ExtAudioFileDispose(g_file);
    g_file = NULL;
    g_loaded = 0;
  }
  g_path[0] = 0;
}

int au_play(const char *path) {
  unload();
  CFURLRef u = CFURLCreateFromFileSystemRepresentation(
      NULL, (const UInt8 *)path, (CFIndex)strlen(path), false);
  if (!u) return 0;
  OSStatus e = ExtAudioFileOpenURL(u, &g_file);
  CFRelease(u);
  if (e != noErr) return 0;

  AudioStreamBasicDescription in = {0};
  UInt32 sz = sizeof in;
  ExtAudioFileGetProperty(g_file, kExtAudioFileProperty_FileDataFormat, &sz, &in);

  AudioStreamBasicDescription out = {0};
  out.mSampleRate = in.mSampleRate;
  out.mFormatID = kAudioFormatLinearPCM;
  out.mFormatFlags = kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked;
  out.mChannelsPerFrame = 2;
  out.mBitsPerChannel = 16;
  out.mBytesPerPacket = 4;
  out.mFramesPerPacket = 1;
  out.mBytesPerFrame = 4;
  if (ExtAudioFileSetProperty(g_file, kExtAudioFileProperty_ClientDataFormat, sizeof out, &out) !=
      noErr) {
    unload();
    return 0;
  }
  g_loaded = 1;

  if (!g_au_ready && !open_device((UInt32)in.mSampleRate)) {
    unload();
    return 0;
  }
  strncpy(g_path, path, sizeof g_path - 1);
  if (AudioOutputUnitStart(g_au) != noErr) {
    unload();
    return 0;
  }
  g_playing = 1;
  return 1;
}

void au_stop(void) { unload(); }

int au_toggle(void) {
  if (!g_loaded) return 0;
  if (g_playing) {
    AudioOutputUnitStop(g_au);
    g_playing = 0;
  } else if (AudioOutputUnitStart(g_au) == noErr) {
    g_playing = 1;
  }
  return 1;
}

int au_playing(void) { return g_playing && g_loaded; }

static double rate(void) {
  if (!g_loaded) return 0.0;
  AudioStreamBasicDescription in = {0};
  UInt32 sz = sizeof in;
  ExtAudioFileGetProperty(g_file, kExtAudioFileProperty_FileDataFormat, &sz, &in);
  return in.mSampleRate > 0 ? in.mSampleRate : 44100.0;
}

double au_pos(void) {
  if (!g_loaded) return 0.0;
  SInt64 frames = 0;
  if (ExtAudioFileTell(g_file, &frames) != noErr) return 0.0;
  return (double)frames / rate();
}

double au_len(void) {
  if (!g_loaded) return 0.0;
  SInt64 frames = 0;
  if (ExtAudioFileGetProperty(g_file, kExtAudioFileProperty_FileLengthFrames, &(UInt32){sizeof frames},
                              &frames) != noErr)
    return 0.0;
  return (double)frames / rate();
}

void au_set_volume(float v) {
  if (g_au_ready) AudioUnitSetParameter(g_au, kHALOutputParam_Volume, kAudioUnitScope_Output, 0, v, 0);
}

const char *au_path(void) { return g_path; }

void au_quit(void) {
  unload();
  if (g_au_ready) {
    AudioComponentInstanceDispose(g_au);
    g_au_ready = 0;
  }
}
