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

static IOSurfaceRef g_surf[2];   /* back/front: writing the surface CA is showing tears */
static CALayer *g_layer;
static NSView *g_view;
static int g_back;            /* index of the buffer the app draws into */
static int g_w, g_h;          /* visible design size (crop) */
static int g_cap_w, g_cap_h;  /* surface size >= visible, allocated once */

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

static CALayer *make_layer(IOSurfaceRef s) {
  CALayer *l = [CALayer layer];
  l.contents = (__bridge id)s;
  l.opaque = YES;
  l.contentsScale = 1.0;              /* 1 design px per point */
  l.contentsGravity = kCAGravityTopLeft; /* 1:1 crop of the cap-sized surface */
  l.magnificationFilter = kCAFilterNearest;
  l.minificationFilter = kCAFilterNearest;
  return l;
}

/* Surface is allocated once for the largest area we may need (screen, in points)
   and never swapped on resize: swapping layer.contents stops CoreAnimation from
   repainting it, which is what produced blank edges on live resize. */
int flp_attach(void *nsview, int w, int h) {
  NSView *v = (__bridge NSView *)nsview;
  if (!v || w < 1 || h < 1) return 0;
  int cw = w;
  int chh = h;
  g_surf[0] = make_surface(cw, chh);
  g_surf[1] = make_surface(cw, chh);
  if (!g_surf[0] || !g_surf[1]) return 0;
  g_back = 0;
  g_view = v;
  g_cap_w = cw;
  g_cap_h = chh;
  g_w = w;
  g_h = h;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  g_layer = make_layer(g_surf[0]);
  g_layer.frame = v.bounds;
  v.layer = g_layer;
  v.wantsLayer = YES;
  [CATransaction commit];
  [CATransaction flush];
  return 1;
}

/* Visible design size only; the surface already covers it. Rarest case (window
   bigger than the screen) grows once. */
int flp_resize(int w, int h) {
  if (!g_view || w < 1 || h < 1) return 0;
  if (w == g_cap_w && h == g_cap_h) {
    g_layer.frame = g_view.bounds;
    return 1;
  }
  int cw = w;
  int chh = h;
  IOSurfaceRef s0 = make_surface(cw, chh);
  IOSurfaceRef s1 = make_surface(cw, chh);
  if (!s0 || !s1) return 0;
  IOSurfaceRef old0 = g_surf[0], old1 = g_surf[1];
  CALayer *l = make_layer(s0);
  g_back = 0;
  l.frame = g_view.bounds;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  g_view.layer = l;
  [CATransaction commit];
  g_layer = l;
  g_layer.frame = g_view.bounds;
  g_surf[0] = s0;
  g_surf[1] = s1;
  g_cap_w = cw;
  g_cap_h = chh;
  g_w = w;
  g_h = h;
  if (old0) CFRelease(old0);
  if (old1) CFRelease(old1);
  return 1;
}

void *flp_buffer(void) {
  return g_surf[g_back] ? IOSurfaceGetBaseAddress(g_surf[g_back]) : NULL;
}
int flp_stride(void) {
  return g_surf[g_back] ? (int)(IOSurfaceGetBytesPerRow(g_surf[g_back]) / 4) : 0;
}
int flp_width(void) { return g_w; }
int flp_height(void) { return g_h; }
/* No [CATransaction flush] here: forcing a synchronous commit every frame makes the
   compositor re-commit the whole window mid-write (flicker in the HUD and at the
   edges). Let CA coalesce; it commits on the next display pass. */
/* Hand the freshly drawn buffer to the layer and flip: the compositor keeps showing
   the other surface, so it can never sample a half-written frame. */
void flp_flush(void) {
  if (!g_layer) return;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  g_layer.contents = (__bridge id)g_surf[g_back];
  [g_layer setContentsChanged];
  [CATransaction commit];
  g_back ^= 1;
}
