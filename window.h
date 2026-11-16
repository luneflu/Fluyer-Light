#ifndef FL_WINDOW_H
#define FL_WINDOW_H

// Thin Cocoa shell. All rendering + logic lives in Rust.
typedef struct fl_window fl_window;

typedef void (*fl_key_cb)(int keycode, unsigned mods, void *ud);
typedef void (*fl_mouse_cb)(int x, int y, void *ud);

// Window is upscaled by the display's integer backing scale; w/h are design pixels.
fl_window *fl_open(int w, int h, const char *title);
void fl_close(fl_window *f);

void *fl_buffer(fl_window *f);   // BGRA8 pixel data, w*h*4 bytes, row-major
int fl_width(fl_window *f);
int fl_height(fl_window *f);
void fl_flush(fl_window *f);     // tell WindowServer the pixels changed

void fl_on_key(fl_window *f, fl_key_cb cb, void *ud);
void fl_on_mouse(fl_window *f, fl_mouse_cb cb, void *ud);

void fl_run(fl_window *f);       // blocks until the app quits

#endif
