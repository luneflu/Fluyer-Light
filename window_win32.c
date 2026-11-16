// window_win32.c - Win32 shell only: window, 32bpp DIB, event forwarding.
// Same contract as window.m: Rust owns the pixels, 0xAARRGGBB each.

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "window.h"

void fl_log(const char *msg) {
  static FILE *f;
  static int tried;
  if (!tried) {
    tried = 1;
    char path[MAX_PATH];
    DWORD n = GetEnvironmentVariableA("TEMP", path, sizeof path);
    if (n && n + 16 < sizeof path) {
      strcat_s(path, sizeof path, "\\fluyer-low.log");
      fopen_s(&f, path, "a");
    }
  }
  fprintf(stderr, "[%9.1f] %s\n", (double)clock() * 1000.0 / CLOCKS_PER_SEC, msg);
  if (f) {
    fprintf(f, "[%9.1f] %s\n", (double)clock() * 1000.0 / CLOCKS_PER_SEC, msg);
    fflush(f);
  }
}

struct fl_window {
  HWND hwnd;
  void *bits;        // grow-only 32bpp BGRA surface, top-down rows
  HBITMAP bmp;       // owns `bits`; deleting it frees them
  int cap_w, cap_h;  // surface size in pixels
  int w, h;          // current view size, design px
  int stride;        // pixels per row
  fl_key_cb key;
  fl_mouse_cb mouse;
  fl_mouse_cb mouse_up;
  fl_resize_cb resize;
  fl_tick_cb tick;
  void *key_ud, *mouse_ud, *resize_ud, *tick_ud;
  void *mouse_up_ud;
  int inset;         // no titlebar strip on Windows; kept for API parity
  UINT_PTR timer;
};

static double g_scale = 1.0; // design px -> physical px (dpi / 96)

typedef HRESULT(WINAPI *dpi_ctx_fn)(HANDLE);
typedef UINT(WINAPI *dpi_win_fn)(HWND);
typedef BOOL(WINAPI *adjust_dpi_fn)(LPRECT, DWORD, BOOL, DWORD, UINT);

static void dpi_init(void) {
  HMODULE user = GetModuleHandleA("user32.dll");
  dpi_ctx_fn set_ctx = (dpi_ctx_fn)(void *)GetProcAddress(user, "SetProcessDpiAwarenessContext");
  if (set_ctx) set_ctx((HANDLE)-4); // PER_MONITOR_AWARE_V2
}

static UINT dpi_of(HWND hwnd) {
  HMODULE user = GetModuleHandleA("user32.dll");
  dpi_win_fn get = (dpi_win_fn)(void *)GetProcAddress(user, "GetDpiForWindow");
  if (get && hwnd) return get(hwnd);
  HDC dc = GetDC(hwnd);
  UINT dpi = dc ? (UINT)GetDeviceCaps(dc, LOGPIXELSX) : 96;
  if (dc) ReleaseDC(hwnd, dc);
  return dpi;
}

// Grow-only surface: resizing smaller never reallocates, so the pointer the
// Rust canvas holds stays valid for the process lifetime.
static int alloc_surface(struct fl_window *f, int w, int h) {
  if (w < 1) w = 1;
  if (h < 1) h = 1;
  if (f->bits && w <= f->cap_w && h <= f->cap_h) return 1;
  int cw = w > f->cap_w ? w : f->cap_w;
  int ch = h > f->cap_h ? h : f->cap_h;
  BITMAPINFO bi;
  memset(&bi, 0, sizeof bi);
  bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  bi.bmiHeader.biWidth = cw;
  bi.bmiHeader.biHeight = -ch; // top-down
  bi.bmiHeader.biPlanes = 1;
  bi.bmiHeader.biBitCount = 32; // BGRA, no palette: matches the canvas layout
  bi.bmiHeader.biCompression = BI_RGB;
  void *bits = NULL;
  HDC dc = GetDC(NULL);
  HBITMAP bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &bits, NULL, 0);
  if (dc) ReleaseDC(NULL, dc);
  if (!bmp || !bits) return 0;
  if (f->bmp) DeleteObject(f->bmp); // frees the old bits
  f->bmp = bmp;
  memset(bits, 0, (size_t)cw * ch * 4);
  f->bits = bits;
  f->cap_w = cw;
  f->cap_h = ch;
  f->stride = cw; // 32bpp rows are already 4-byte aligned
  return 1;
}

static void blit(struct fl_window *f) {
  if (!f->bits) return;
  HDC dc = GetDC(f->hwnd);
  if (!dc) return;
  RECT rc;
  GetClientRect(f->hwnd, &rc);
  BITMAPINFO bi;
  memset(&bi, 0, sizeof bi);
  bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  bi.bmiHeader.biWidth = f->cap_w;
  bi.bmiHeader.biHeight = -f->cap_h;
  bi.bmiHeader.biPlanes = 1;
  bi.bmiHeader.biBitCount = 32;
  bi.bmiHeader.biCompression = BI_RGB;

  SetStretchBltMode(dc, COLORONCOLOR); // nearest neighbour: pixel art stays crisp
  StretchDIBits(dc, 0, 0, rc.right, rc.bottom, 0, 0, f->w, f->h, f->bits, &bi, DIB_RGB_COLORS,
                SRCCOPY);
  ReleaseDC(f->hwnd, dc);
}

static int design_w(struct fl_window *f, int phys) { return (int)(phys / g_scale + 0.5); }

static void push_resize(struct fl_window *f) {
  RECT rc;
  GetClientRect(f->hwnd, &rc);
  int w = design_w(f, rc.right), h = design_w(f, rc.bottom);
  if (w < 1) w = 1;
  if (h < 1) h = 1;
  if (w == f->w && h == f->h) return;
  f->w = w;
  f->h = h;
  { char b[64]; sprintf_s(b, sizeof b, "resize %dx%d", w, h); fl_log(b); }
  if (!alloc_surface(f, w, h)) return;
  if (f->resize) f->resize(w, h, f->resize_ud);
}

static LRESULT CALLBACK wnd_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
  struct fl_window *f = (struct fl_window *)(void *)GetWindowLongPtrA(hwnd, GWLP_USERDATA);
  switch (msg) {
  case WM_NCCREATE:
    SetWindowLongPtrA(hwnd, GWLP_USERDATA, (LONG_PTR)((CREATESTRUCTA *)lp)->lpCreateParams);
    return TRUE;
  case WM_PAINT: {
    PAINTSTRUCT ps;
    HDC dc = BeginPaint(hwnd, &ps);
    (void)dc;
    if (f) blit(f);
    EndPaint(hwnd, &ps);
    return 0;
  }
  case WM_ERASEBKGND:
    return 1; // we always paint the whole client area
  case WM_SIZE:
    if (f && wp != SIZE_MINIMIZED) push_resize(f);
    return 0;
  case WM_DPICHANGED:
    g_scale = (double)HIWORD(wp) / 96.0;
    if (f) {
      RECT *r = (RECT *)lp;
      SetWindowPos(hwnd, NULL, r->left, r->top, r->right - r->left, r->bottom - r->top,
                   SWP_NOZORDER | SWP_NOACTIVATE);
    }
    return 0;
  case WM_GETMINMAXINFO: {
    MINMAXINFO *mm = (MINMAXINFO *)lp;
    mm->ptMinTrackSize.x = (LONG)(64 * g_scale);
    mm->ptMinTrackSize.y = (LONG)(48 * g_scale);
    return 0;
  }
  case WM_TIMER:
    if (f && f->tick) f->tick(f->tick_ud);
    return 0;
  case WM_KEYDOWN:
  case WM_SYSKEYDOWN:
    if (f && f->key) {
      unsigned mods = 0;
      if (GetKeyState(VK_CONTROL) & 0x8000) mods |= 1u << 0; // cmd == ctrl on Windows
      if (GetKeyState(VK_SHIFT) & 0x8000) mods |= 1u << 1;
      if (GetKeyState(VK_CONTROL) & 0x8000) mods |= 1u << 2;
      if (GetKeyState(VK_MENU) & 0x8000) mods |= 1u << 3;
      f->key((int)wp, mods, f->key_ud);
    }
    if (wp == VK_ESCAPE || wp == VK_SPACE || wp == VK_RETURN || wp == VK_UP || wp == VK_DOWN)
      return 0; // no beep / dialog handling for keys we own
    break;
  case WM_LBUTTONDOWN:
    if (f && f->mouse) {
      int x = (int)((short)LOWORD(lp) / g_scale);
      int y = (int)((short)HIWORD(lp) / g_scale); // client coords are already top-down
      f->mouse(x, y, f->mouse_ud);
    }
    return 0;
  case WM_LBUTTONUP:
    if (f && f->mouse_up) f->mouse_up(0, 0, f->mouse_up_ud);
    return 0;
  case WM_CLOSE:
    PostQuitMessage(0);
    return 0;
  case WM_DESTROY:
    PostQuitMessage(0);
    return 0;
  }
  return DefWindowProcA(hwnd, msg, wp, lp);
}

static void style_caption(HWND hwnd) {
  HMODULE dwm = LoadLibraryA("dwmapi.dll");
  if (dwm) {
    typedef HRESULT(WINAPI * dwm_set_fn)(HWND, DWORD, LPCVOID, DWORD);
    dwm_set_fn set = (dwm_set_fn)(void *)GetProcAddress(dwm, "DwmSetWindowAttribute");
    if (set) {
      BOOL on = TRUE;
      set(hwnd, 20, &on, sizeof on); // DWMWA_USE_IMMERSIVE_DARK_MODE
      COLORREF bg = RGB(0x18, 0x18, 0x18);
      set(hwnd, 35, &bg, sizeof bg); // DWMWA_CAPTION_COLOR: same grey as the app
    }
  }
  // Icon gone, title text gone: caption buttons and the drag strip are all that is left.
  LONG_PTR ex = GetWindowLongPtrA(hwnd, GWL_EXSTYLE);
  SetWindowLongPtrA(hwnd, GWL_EXSTYLE, ex | WS_EX_DLGMODALFRAME);
  SendMessageA(hwnd, WM_SETICON, ICON_SMALL, 0);
  SendMessageA(hwnd, WM_SETICON, ICON_BIG, 0);
  SetWindowTextA(hwnd, "");
  SetWindowPos(hwnd, NULL, 0, 0, 0, 0,
               SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
}

fl_window *fl_open(int w, int h, const char *title) {
  dpi_init();

  static int registered;
  HINSTANCE inst = GetModuleHandleA(NULL);
  if (!registered) {
    WNDCLASSA wc;
    memset(&wc, 0, sizeof wc);
    wc.lpfnWndProc = wnd_proc;
    wc.hInstance = inst;
    wc.hCursor = LoadCursorA(NULL, (LPCSTR)IDC_ARROW);
    wc.hbrBackground = NULL;
    wc.lpszClassName = "fluyer-low";
    if (!RegisterClassA(&wc)) return NULL;
    registered = 1;
  }

  struct fl_window *f = (struct fl_window *)calloc(1, sizeof *f);
  if (!f) return NULL;
  f->w = w;
  f->h = h;

  DWORD style = WS_OVERLAPPEDWINDOW;
  int pw = (int)(w * g_scale), ph = (int)(h * g_scale);
  RECT rc = {0, 0, pw, ph};
  HMODULE user = GetModuleHandleA("user32.dll");
  adjust_dpi_fn adj = (adjust_dpi_fn)(void *)GetProcAddress(user, "AdjustWindowRectExForDpi");
  if (adj) adj(&rc, style, FALSE, 0, (UINT)(g_scale * 96));
  else AdjustWindowRectEx(&rc, style, FALSE, 0);

  f->hwnd = CreateWindowExA(0, "fluyer-low", title, style, CW_USEDEFAULT, CW_USEDEFAULT,
                            rc.right - rc.left, rc.bottom - rc.top, NULL, NULL, inst, f);
  if (!f->hwnd) {
    free(f);
    return NULL;
  }

  g_scale = (double)dpi_of(f->hwnd) / 96.0;
  style_caption(f->hwnd);
  if (!alloc_surface(f, w, h)) {
    DestroyWindow(f->hwnd);
    free(f);
    return NULL;
  }

  ShowWindow(f->hwnd, SW_SHOW);
  UpdateWindow(f->hwnd);
  SetFocus(f->hwnd);
  fl_log("open");
  return f;
}

void fl_close(fl_window *f) {
  if (!f) return;
  if (f->timer) KillTimer(f->hwnd, f->timer);
  if (f->hwnd) DestroyWindow(f->hwnd);
  if (f->bmp) DeleteObject(f->bmp);
  free(f);
}

void *fl_buffer(fl_window *f) { return f ? f->bits : NULL; }
int fl_top_inset(fl_window *f) { return f ? f->inset : 0; }
int fl_width(fl_window *f) { return f ? f->w : 0; }
int fl_height(fl_window *f) { return f ? f->h : 0; }

void fl_flush(fl_window *f) {
  if (!f) return;
  InvalidateRect(f->hwnd, NULL, FALSE);
  UpdateWindow(f->hwnd);
}

void *fl_resize_ensure(fl_window *f, int w, int h, int *stride) {
  if (!f || w < 1 || h < 1) {
    fl_log("ensure REJECT");
    return NULL;
  }
  f->w = w;
  f->h = h;
  if (!alloc_surface(f, w, h)) return NULL;
  if (stride) *stride = f->stride;
  return f->bits;
}

void fl_on_key(fl_window *f, fl_key_cb cb, void *ud) { f->key = cb; f->key_ud = ud; }
void fl_on_mouse(fl_window *f, fl_mouse_cb cb, void *ud) { f->mouse = cb; f->mouse_ud = ud; }
void fl_on_mouse_up(fl_window *f, fl_mouse_cb cb, void *ud) {
  f->mouse_up = cb;
  f->mouse_up_ud = ud;
}
void fl_on_resize(fl_window *f, fl_resize_cb cb, void *ud) { f->resize = cb; f->resize_ud = ud; }

void fl_on_tick(fl_window *f, fl_tick_cb cb, void *ud, int ms) {
  if (!f) return;
  f->tick = cb;
  f->tick_ud = ud;
  if (f->timer) KillTimer(f->hwnd, f->timer);
  f->timer = SetTimer(f->hwnd, 1, (UINT)(ms < 1 ? 1 : ms), NULL);
}

void fl_run(fl_window *f) {
  (void)f;
  MSG msg;
  while (GetMessageA(&msg, NULL, 0, 0) > 0) {
    TranslateMessage(&msg);
    DispatchMessageA(&msg);
  }
}
