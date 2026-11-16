// window.m — Cocoa shell only: window, IOSurface, event forwarding.
// No drawing, no app logic. Rust owns the pixels.
#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <IOSurface/IOSurface.h>
#include <malloc/malloc.h>
#include <unistd.h>
#include "window.h"

@interface CALayer (ContentsChanged)
- (void)setContentsChanged;
@end

struct fl_window {
  int w, h, scale;
  IOSurfaceRef surface;
  CALayer *layer;
  fl_key_cb key;
  fl_mouse_cb mouse;
  void *ud;
};

@interface FLView : NSView
@property(nonatomic, assign) struct fl_window *owner;
@end

@implementation FLView
- (BOOL)acceptsFirstResponder { return YES; }
- (BOOL)isOpaque { return YES; }

- (void)keyDown:(NSEvent *)e {
  struct fl_window *f = self.owner;
  if (!f || !f->key) return;
  unsigned mods = 0;
  NSEventModifierFlags m = e.modifierFlags;
  if (m & NSEventModifierFlagCommand) mods |= 1u << 0;
  if (m & NSEventModifierFlagShift) mods |= 1u << 1;
  if (m & NSEventModifierFlagControl) mods |= 1u << 2;
  if (m & NSEventModifierFlagOption) mods |= 1u << 3;
  f->key((int)e.keyCode, mods, f->ud);
}

- (void)mouseDown:(NSEvent *)e {
  struct fl_window *f = self.owner;
  if (!f || !f->mouse) return;
  NSPoint p = [self convertPoint:e.locationInWindow fromView:nil];
  f->mouse((int)p.x / f->scale, f->h - (int)p.y / f->scale, f->ud);
}
@end

@interface FLWindow : NSWindow
@end

@implementation FLWindow
- (BOOL)canBecomeKeyWindow { return YES; }
- (BOOL)canBecomeMainWindow { return YES; }
@end

fl_window *fl_open(int w, int h, const char *title) {
  NSApplication *app = [NSApplication sharedApplication];
  [app setActivationPolicy:NSApplicationActivationPolicyRegular];

  // DPI: one design pixel -> backingScaleFactor device pixels, integer only.
  int scale = (int)lround([NSScreen mainScreen].backingScaleFactor);
  if (scale < 1) scale = 1;
  if (scale > 4) scale = 4;
  struct fl_window *f = calloc(1, sizeof *f);
  f->w = w;
  f->h = h;
  f->scale = scale;

  f->surface = IOSurfaceCreate((__bridge CFDictionaryRef)@{
    (id)kIOSurfaceWidth: @(w),
    (id)kIOSurfaceHeight: @(h),
    (id)kIOSurfaceBytesPerElement: @(4),
    (id)kIOSurfaceBytesPerRow: @(w * 4),
    (id)kIOSurfaceAllocSize: @(w * h * 4),
    (id)kIOSurfacePixelFormat: @(0x42475241)   // 'BGRA'
  });
  if (!f->surface) { free(f); return NULL; }

  FLWindow *win = [[FLWindow alloc]
      initWithContentRect:NSMakeRect(0, 0, w * scale, h * scale)
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskMiniaturizable
                  backing:NSBackingStoreBuffered defer:NO];
  [win setTitle:@(title)];
  win.movableByWindowBackground = YES;
  win.releasedWhenClosed = NO;

  FLView *v = [[FLView alloc] initWithFrame:NSMakeRect(0, 0, w * scale, h * scale)];
  v.owner = f;
  f->layer = [CALayer layer];
  f->layer.contents = (__bridge id)f->surface;
  f->layer.opaque = YES;
  f->layer.magnificationFilter = kCAFilterNearest;
  f->layer.minificationFilter = kCAFilterNearest;
  f->layer.contentsScale = 1.0;
  f->layer.contentsGravity = kCAGravityResize;
  f->layer.bounds = CGRectMake(0, 0, w, h);   // design space, compositor does integer upscale
  v.layer = f->layer;
  v.wantsLayer = YES;

  [win setContentView:v];
  [win center];
  [win makeKeyAndOrderFront:nil];
  [win makeFirstResponder:v];
  [app activateIgnoringOtherApps:YES];

  malloc_zone_pressure_relief(NULL, 0);
  return f;
}

void fl_close(fl_window *f) {
  if (!f) return;
  if (f->surface) CFRelease(f->surface);
  free(f);
}

void *fl_buffer(fl_window *f) { return IOSurfaceGetBaseAddress(f->surface); }
int fl_width(fl_window *f) { return f->w; }
int fl_height(fl_window *f) { return f->h; }

void fl_flush(fl_window *f) { [f->layer setContentsChanged]; }

void fl_on_key(fl_window *f, fl_key_cb cb, void *ud) { f->key = cb; f->ud = ud; }
void fl_on_mouse(fl_window *f, fl_mouse_cb cb, void *ud) { f->mouse = cb; f->ud = ud; }

void fl_run(fl_window *f) { [NSApp run]; }
