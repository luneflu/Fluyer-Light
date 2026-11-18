/* present: macOS pixel presenter. One BGRA IOSurface at design resolution as the
   view's layer contents; the compositor nearest-upscales to the backing store.
   Cost: w*h*4 bytes, single buffer, no per-frame allocation. */
#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <IOSurface/IOSurface.h>

/* -setContentsChanged is SPI; declare it so the layer repaints on every flush. */
@interface CALayer (FLPresent)
- (void)setContentsChanged;
@end

static IOSurfaceRef g_surf;
static CALayer *g_layer;
static NSView *g_view;
static int g_w, g_h;      /* surface size = design px */

/* 'BGRA' fourcc: @('BGRA') boxes to char 65, so use the raw int. */
static IOSurfaceRef make_surface(int w, int h) {
  return IOSurfaceCreate((__bridge CFDictionaryRef)@{
    (id)kIOSurfaceWidth: @(w),
    (id)kIOSurfaceHeight: @(h),
    (id)kIOSurfaceBytesPerElement: @(4),
    (id)kIOSurfaceBytesPerRow: @(w * 4),
    (id)kIOSurfaceAllocSize: @(w * h * 4),
    (id)kIOSurfacePixelFormat: @(0x42475241)
  });
}

/* Attach to a winit-created NSView. Surface is grow-only: swapping layer.contents
   stops CoreAnimation from repainting it, so it is never replaced. */
int flp_attach(void *nsview, int w, int h) {
  NSView *v = (__bridge NSView *)nsview;
  if (!v || w < 1 || h < 1) return 0;
  g_surf = make_surface(w, h);
  if (!g_surf) return 0;
  g_view = v;
  g_w = w;
  g_h = h;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  g_layer = [CALayer layer];
  g_layer.contents = (__bridge id)g_surf;
  g_layer.opaque = YES;
  g_layer.contentsScale = 1.0;            /* 1 design px per point */
  g_layer.contentsGravity = kCAGravityTopLeft;
  g_layer.magnificationFilter = kCAFilterNearest;
  g_layer.minificationFilter = kCAFilterNearest;
  g_layer.frame = v.bounds;
  v.layer = g_layer;
  v.wantsLayer = YES;
  [CATransaction commit];
  return 1;
}

/* Design size for the app; keeps the crop 1:1. */
/* New size = new surface + new layer. Assigning layer.contents in place stops
   CoreAnimation from repainting the surface, so the layer is replaced instead. */
int flp_resize(int w, int h) {
  if (!g_view || w < 1 || h < 1) return 0;
  IOSurfaceRef s = make_surface(w, h);
  if (!s) return 0;
  IOSurfaceRef old = g_surf;
  CALayer *l = [CALayer layer];
  l.contents = (__bridge id)s;
  l.opaque = YES;
  l.contentsScale = 1.0;
  l.contentsGravity = kCAGravityTopLeft;
  l.magnificationFilter = kCAFilterNearest;
  l.minificationFilter = kCAFilterNearest;
  l.frame = g_view.bounds;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  g_view.layer = l;
  [CATransaction commit];
  g_layer = l;
  g_surf = s;
  g_w = w;
  g_h = h;
  if (old) CFRelease(old);   /* after the swap; the layer no longer references it */
  return 1;
}

void *flp_buffer(void) { return g_surf ? IOSurfaceGetBaseAddress(g_surf) : NULL; }
int flp_stride(void) {
  return g_surf ? (int)(IOSurfaceGetBytesPerRow(g_surf) / 4) : 0;
}
int flp_width(void) { return g_w; }
int flp_height(void) { return g_h; }
void flp_flush(void) { if (g_layer) [g_layer setContentsChanged]; }
