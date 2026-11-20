// layout: every rect the view needs, computed from the window size alone.

use fluyer_pxui::gfx::Rect;
use fluyer_pxui::theme::{
    BAR_GAP, BAR_HEIGHT, BAR_PADDING, SEEK_HEIGHT, TRANSPORT_SIZE,
};
use fluyer_pxui::widgets::seek::SeekBar;

pub struct Layout {
    pub w: i32,
    pub h: i32,
    pub inset: i32,
    pub list_y: i32,
    pub list_view_h: i32,
    pub seek: SeekBar,
    pub row_y: i32,
    pub icon: i32,
    /// Rect of the [prev][play][next] cluster, right-aligned.
    pub transport: Rect,
}

impl Layout {
    pub fn new(w: i32, h: i32, inset: i32) -> Self {
        let seek = SeekBar {
            rect: Rect::new(
                BAR_PADDING,
                h - BAR_HEIGHT + BAR_PADDING,
                (w - 2 * BAR_PADDING).max(0),
                SEEK_HEIGHT,
            ),
        };
        let row_y = seek.rect.y + SEEK_HEIGHT + BAR_GAP;
        let icon = TRANSPORT_SIZE;
        let transport_w = 3 * icon + 2 * BAR_GAP;
        let transport = Rect::new(
            (w - BAR_PADDING - transport_w).max(0),
            row_y,
            transport_w,
            icon,
        );
        Self {
            w,
            h,
            inset,
            list_y: inset,
            list_view_h: (h - BAR_HEIGHT - inset).max(0),
            seek,
            row_y,
            icon,
            transport,
        }
    }

    /// x of the i-th transport slot: 0 = prev, 1 = play, 2 = next.
    pub fn slot(&self, i: i32) -> Rect {
        Rect::new(
            self.transport.x + i * (self.icon + BAR_GAP),
            self.transport.y,
            self.icon,
            self.icon,
        )
    }

    /// Width left for the track name, to the left of the transport cluster.
    pub fn name_room(&self) -> i32 {
        self.transport.x - BAR_GAP - BAR_PADDING
    }
}
