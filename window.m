// window.m — Cocoa shell only: window, IOSurface, event forwarding.
// No drawing, no app logic. Rust owns the pixels.
#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <IOSurface/IOSurface.h>
#include <malloc/malloc.h>
#include <stdarg.h>
#include <time.h>
#include <stdio.h>
#include "window.h"

void fl_log(const char *msg) {
  static FILE *f;
  if (!f) f = fopen("/tmp/fluyer-low.log", "a");
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  double ms = ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
  fprintf(stderr, "[%9.1f] %s\n", ms, msg);
  if (f) {
    fprintf(f, "[%9.1f] %s\n", ms, msg);
    fflush(f);
  }
}

#define FLLOG(...) do { char _b[256]; snprintf(_b, sizeof _b, __VA_ARGS__); fl_log(_b); } while (0)

@interface CALayer (ContentsChanged)
- (void)setContentsChanged;
@end

struct fl_window {
  int w, h;       // current view size (design px)
  int cap_w, cap_h; // surface size; never reallocated
  IOSurfaceRef surface;
  CALayer *layer;
  fl_key_cb key;
  fl_mouse_cb mouse;
  fl_mouse_cb mouse_up;
  void *mouse_up_ud;
  fl_resize_cb resize;
  fl_tick_cb tick;
  void *tick_ud;
  void *timer;   // __bridge NSTimer
  void *view;   // __bridge FLView, for layout during resize
  int inset;    // titlebar strip above the content, kept clear for traffic lights
  int pend_w, pend_h, scheduled;
  IOSurfaceRef pending;
  void *key_ud;
  void *mouse_ud;
  void *resize_ud;
};

@interface FLView : NSView
@property(nonatomic, assign) struct fl_window *owner;
@end

@implementation FLView
- (BOOL)acceptsFirstResponder { return YES; }
/* Layer-backed views default to NO, which kills background dragging. */
- (BOOL)mouseDownCanMoveWindow { return YES; }
- (BOOL)isOpaque { return YES; }

/* Live resize stretches the existing surface (nearest); the crisp surface is
   reallocated once when the drag ends, so no half-painted frame is ever shown. */
/* Layer-hosting view: AppKit does not resize our layer for us. */
- (void)layout {
  [super layout];
  struct fl_window *f = self.owner;
  if (f && f->layer) f->layer.frame = self.bounds;
}

- (void)viewDidEndLiveResize {
  [super viewDidEndLiveResize];
  FLLOG("liveResize END bounds=%.0fx%.0f", self.bounds.size.width, self.bounds.size.height);
  [self scheduleResize:self.bounds.size];
}

- (void)scheduleResize:(NSSize)s {
  struct fl_window *f = self.owner;
  if (!f || !f->resize) return;
  int w = (int)lround(s.width);
  int h = (int)lround(s.height);
  if (w < 1 || h < 1) return;
  if (w == f->w && h == f->h) return;
  FLLOG("schedule %dx%d (surface %dx%d)", w, h, f->w, f->h);
  f->pend_w = w;
  f->pend_h = h;
  if (f->scheduled) return;
  f->scheduled = 1;
  dispatch_async(dispatch_get_main_queue(), ^{
    f->scheduled = 0;
    FLLOG("dispatch resize %dx%d", f->pend_w, f->pend_h);
    if (!f->resize) return;
    if (f->pend_w == f->w && f->pend_h == f->h) return;
    f->resize(f->pend_w, f->pend_h, f->resize_ud);
  });
}

- (void)setFrameSize:(NSSize)s {
  [super setFrameSize:s];
  struct fl_window *f = self.owner;
  FLLOG("setFrameSize %.0fx%.0f live=%d surface=%dx%d", s.width, s.height,
        (int)self.inLiveResize, f ? f->w : 0, f ? f->h : 0);
  if (self.inLiveResize) return;   // handled in viewDidEndLiveResize
  [self scheduleResize:s];
}

- (void)keyDown:(NSEvent *)e {
  struct fl_window *f = self.owner;
  if (!f || !f->key) return;
  unsigned mods = 0;
  NSEventModifierFlags m = e.modifierFlags;
  if (m & NSEventModifierFlagCommand) mods |= 1u << 0;
  if (m & NSEventModifierFlagShift) mods |= 1u << 1;
  if (m & NSEventModifierFlagControl) mods |= 1u << 2;
  if (m & NSEventModifierFlagOption) mods |= 1u << 3;
  f->key((int)e.keyCode, mods, f->key_ud);
}

- (void)mouseDown:(NSEvent *)e {
  struct fl_window *f = self.owner;
  if (!f) return;
  NSPoint p = [self convertPoint:e.locationInWindow fromView:nil];
  FLLOG("mouseDown click=%ld p=%.0f,%.0f bounds=%.0fx%.0f inset=%d", (long)e.clickCount,
        p.x, p.y, self.bounds.size.width, self.bounds.size.height, f->inset);
  /* Double-click the titlebar strip (full-size content view has no real titlebar). */
  if (e.clickCount == 2 && p.y >= self.bounds.size.height - f->inset) {
    FLLOG("titlebar double-click: zoom");
    [self.window zoom:nil];
    return;
  }
  if (!f->mouse) return;
  f->mouse((int)p.x, f->h - (int)p.y, f->mouse_ud);
}

- (void)mouseUp:(NSEvent *)e {
  struct fl_window *f = self.owner;
  if (f && f->mouse_up) f->mouse_up(0, 0, f->mouse_up_ud);
}
@end

@interface FLWindow : NSWindow
@end

@implementation FLWindow
- (BOOL)canBecomeKeyWindow { return YES; }
- (BOOL)canBecomeMainWindow { return YES; }
@end

static IOSurfaceRef make_surface(int w, int h) {
  return IOSurfaceCreate((__bridge CFDictionaryRef)@{
    (id)kIOSurfaceWidth: @(w),
    (id)kIOSurfaceHeight: @(h),
    (id)kIOSurfaceBytesPerElement: @(4),
    (id)kIOSurfaceBytesPerRow: @(w * 4),
    (id)kIOSurfaceAllocSize: @(w * h * 4),
    /* 'BGRA' 8-bit colour, byte order matches the canvas (0xAARRGGBB).
       @('BGRA') boxes to char 65, hence the raw fourcc. */
    (id)kIOSurfacePixelFormat: @(0x42475241)
  });
}

fl_window *fl_open(int w, int h, const char *title) {
  NSApplication *app = [NSApplication sharedApplication];
  [app setActivationPolicy:NSApplicationActivationPolicyRegular];

  struct fl_window *f = calloc(1, sizeof *f);
  f->w = w;
  f->h = h;
  /* One surface for the process lifetime: swapping layer.contents on resize
     stops CA from repainting it, so allocate the largest area we may need. */
  NSRect screen = [NSScreen mainScreen].frame;
  int cw = (int)screen.size.width;
  int ch = (int)screen.size.height;
  if (cw < w) cw = w;
  if (ch < h) ch = h;
  f->cap_w = cw;
  f->cap_h = ch;
  f->surface = make_surface(cw, ch);
  if (!f->surface) { free(f); return NULL; }

  FLWindow *win = [[FLWindow alloc]
      initWithContentRect:NSMakeRect(0, 0, w, h)
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable
                  backing:NSBackingStoreBuffered defer:NO];
  [win setTitle:@(title)];
  /* Standard titlebar height, measured before the content view takes it over. */
  int inset = (int)(win.frame.size.height - win.contentLayoutRect.size.height);
  /* Transparent full-size titlebar: traffic lights only, no title bar chrome. The
     content view now spans the whole frame, so the canvas is h + inset tall and the
     app keeps `inset` design px clear at the top. */
  win.styleMask |= NSWindowStyleMaskFullSizeContentView;
  win.titlebarAppearsTransparent = YES;
  win.titleVisibility = NSWindowTitleHidden;
  win.movableByWindowBackground = YES;   // drag anywhere in the client area moves the window
  win.releasedWhenClosed = NO;
  win.minSize = NSMakeSize(64, 48);
  f->inset = inset;
  f->w = w;
  f->h = (int)win.frame.size.height;

  FLView *v = [[FLView alloc] initWithFrame:NSMakeRect(0, 0, w, f->h)];
  v.owner = f;
  f->view = (__bridge void *)v;
  f->layer = [CALayer layer];
  f->layer.contentsFormat = kCAContentsFormatRGBA8Uint;   // matches 'BGRA' bytes
  f->layer.contents = (__bridge id)f->surface;
  f->layer.opaque = YES;
  f->layer.magnificationFilter = kCAFilterNearest;
  f->layer.minificationFilter = kCAFilterNearest;
  f->layer.contentsScale = 1.0;   // 1 design px per point; AppKit scales to the backing store
  f->layer.contentsGravity = kCAGravityTopLeft;   // 1:1 crop, never stretch
  v.layer = f->layer;
  v.wantsLayer = YES;

  [win setContentView:v];
  [win center];
  [win makeKeyAndOrderFront:nil];
  FLLOG("open %dx%d (requested %dx%d inset %d) cap %dx%d", f->w, f->h, w, h, inset, cw, ch);
  [win makeFirstResponder:v];
  [app activateIgnoringOtherApps:YES];

  malloc_zone_pressure_relief(NULL, 0);
  return f;
}

void fl_close(fl_window *f) {
  if (!f) return;
  if (f->pending) CFRelease(f->pending);
  if (f->surface) CFRelease(f->surface);
  free(f);
}

void *fl_buffer(fl_window *f) { return IOSurfaceGetBaseAddress(f->surface); }
int fl_top_inset(fl_window *f) { return f ? f->inset : 0; }

int fl_width(fl_window *f) { return f->w; }
int fl_height(fl_window *f) { return f->h; }

void fl_flush(fl_window *f) { [f->layer setContentsChanged]; }

void *fl_resize_begin(fl_window *f, int w, int h) {
  if (!f || w < 1 || h < 1) { FLLOG("begin REJECT %dx%d", w, h); return NULL; }
  IOSurfaceRef s = make_surface(w, h);
  if (!s) { FLLOG("begin FAIL surface %dx%d", w, h); return NULL; }
  FLLOG("begin %dx%d ok", w, h);
  if (f->pending) CFRelease(f->pending);
  f->pending = s;
  f->w = w;
  f->h = h;
  return IOSurfaceGetBaseAddress(s);
}

void *fl_resize_ensure(fl_window *f, int w, int h, int *stride) {
  if (!f || w < 1 || h < 1) {
    FLLOG("ensure REJECT %dx%d", w, h);
    return NULL;
  }
  if (w > f->cap_w) { FLLOG("ensure clamp w %d -> %d", w, f->cap_w); w = f->cap_w; }
  if (h > f->cap_h) { FLLOG("ensure clamp h %d -> %d", h, f->cap_h); h = f->cap_h; }
  f->w = w;
  f->h = h;
  NSView *v = (__bridge NSView *)f->view;
  f->layer.frame = v.bounds;
  if (stride) *stride = (int)(IOSurfaceGetBytesPerRow(f->surface) / (int)IOSurfaceGetBytesPerElement(f->surface));
  FLLOG("ensure %dx%d cap=%dx%d stride=%d", w, h, f->cap_w, f->cap_h,
        stride ? *stride : -1);
  return IOSurfaceGetBaseAddress(f->surface);
}

void fl_on_key(fl_window *f, fl_key_cb cb, void *ud) { f->key = cb; f->key_ud = ud; }
void fl_on_mouse(fl_window *f, fl_mouse_cb cb, void *ud) { f->mouse = cb; f->mouse_ud = ud; }
void fl_on_mouse_up(fl_window *f, fl_mouse_cb cb, void *ud) {
  f->mouse_up = cb;
  f->mouse_up_ud = ud;
}
void fl_on_resize(fl_window *f, fl_resize_cb cb, void *ud) { f->resize = cb; f->resize_ud = ud; }

@interface FLTickTarget : NSObject
@property(nonatomic, assign) struct fl_window *owner;
@end

@implementation FLTickTarget
- (void)fire:(NSTimer *)t {
  struct fl_window *f = self.owner;
  if (f && f->tick) f->tick(f->tick_ud);
}
@end

void fl_on_tick(fl_window *f, fl_tick_cb cb, void *ud, int ms) {
  if (!f) return;
  f->tick = cb;
  f->tick_ud = ud;
  if (f->timer) [(__bridge NSTimer *)f->timer invalidate];
  FLTickTarget *t = [FLTickTarget new];
  t.owner = f;
  NSTimer *timer = [NSTimer scheduledTimerWithTimeInterval:(ms / 1000.0)
                                                   target:t
                                                 selector:@selector(fire:)
                                                 userInfo:nil
                                                  repeats:YES];
  [[NSRunLoop mainRunLoop] addTimer:timer forMode:NSRunLoopCommonModes];
  f->timer = (__bridge_retained void *)timer;
}

void fl_run(fl_window *f) { [NSApp run]; }
