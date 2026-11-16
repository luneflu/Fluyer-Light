#ifndef FL_WINDOW_H
#define FL_WINDOW_H

// Thin Cocoa shell. All rendering + logic lives in Rust.
// Design space = window points. Retina/DPI is handled by AppKit, not by us.
typedef struct fl_window fl_window;

typedef void (*fl_key_cb)(int keycode, unsigned mods, void *ud);
typedef void (*fl_mouse_cb)(int x, int y, void *ud);
typedef void (*fl_resize_cb)(int w, int h, void *ud);
typedef void (*fl_tick_cb)(void *ud);   // fires on the main run loop while fl_run blocks

// Diagnostics: window/resize trace, appended to /tmp/fluyer-low.log and stderr.
void fl_log(const char *msg);

fl_window *fl_open(int w, int h, const char *title);
void fl_close(fl_window *f);

void *fl_buffer(fl_window *f);   // BGRA8 pixel data, w*h*4 bytes, row-major
int fl_width(fl_window *f);
int fl_height(fl_window *f);
int fl_top_inset(fl_window *f);   // titlebar height kept clear above the content
void fl_flush(fl_window *f);     // tell WindowServer the pixels changed

// Grow-only surface: resizing a window smaller never reallocates and never
// swaps layer.contents (that swap is what CA refuses to repaint).
// Returns pixel data for a view of w*h; *stride is pixels per row (>= w).
void *fl_resize_ensure(fl_window *f, int w, int h, int *stride);

void fl_on_key(fl_window *f, fl_key_cb cb, void *ud);
void fl_on_mouse(fl_window *f, fl_mouse_cb cb, void *ud);
void fl_on_resize(fl_window *f, fl_resize_cb cb, void *ud);
void fl_on_tick(fl_window *f, fl_tick_cb cb, void *ud, int ms);

void fl_run(fl_window *f);       // blocks until the app quits

#endif
