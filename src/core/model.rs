// model: the app's semantic state and the actions that change it. No pixels, no
// geometry, no OS. A UI projects this; it never owns it.

use crate::core::player::Player;

pub struct Model {
    pub tracks: Vec<String>,
    pub selected: Option<usize>, // list cursor
    pub scroll: usize,           // first visible row
    pub visible: usize,          // rows that fit; set from the layout on resize
    pub player: Player,
}

/// Everything the app can be told to do, independent of input devices.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Select(usize),
    Step(i32), // prev/next relative to the playing track
    Move(i32),     // move the list cursor only
    Scroll(i32),   // wheel rows: positive scrolls down
    Toggle,    // play/pause
    Play,      // start the selected track
    Seek(f64), // 0..1
    Tick,
    Quit,
}

/// What a redraw has to cover.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Changed {
    Nothing,
    Progress, // seek strip only
    Full,
    Quit,
}

impl Model {
    pub fn new(tracks: Vec<String>) -> Self {
        let selected = if tracks.is_empty() { None } else { Some(0) };
        Self { tracks, selected, scroll: 0, visible: 1, player: Player::new() }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// How many rows fit right now. Clamps the scroll when the window changes.
    pub fn set_visible(&mut self, visible: usize) {
        self.visible = visible.max(1);
        self.clamp_scroll();
    }

    pub fn max_scroll(&self) -> usize {
        self.len().saturating_sub(self.visible)
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.max_scroll());
    }

    /// Keep the cursor on screen after it moves.
    fn reveal(&mut self) {
        let Some(i) = self.selected else { return };
        if i < self.scroll {
            self.scroll = i;
        } else if i >= self.scroll + self.visible {
            self.scroll = i + 1 - self.visible;
        }
        self.clamp_scroll();
    }

    pub fn current_path(&self) -> Option<&String> {
        self.player.current.and_then(|i| self.tracks.get(i))
    }

    pub fn selected_path(&self) -> Option<(usize, &String)> {
        let i = self.selected?;
        Some((i, self.tracks.get(i)?))
    }

    pub fn update(&mut self, a: Action) -> Changed {
        match a {
            Action::Quit => Changed::Quit,
            Action::Move(d) => {
                if self.move_cursor(d) {
                    self.reveal();
                    Changed::Full
                } else {
                    Changed::Nothing
                }
            }
            Action::Scroll(d) => {
                let before = self.scroll;
                let next = self.scroll as i32 + d;
                self.scroll = next.clamp(0, self.max_scroll() as i32) as usize;
                if self.scroll == before {
                    Changed::Nothing
                } else {
                    Changed::Full
                }
            }
            Action::Step(d) => {
                let n = self.len() as i32;
                if n == 0 {
                    return Changed::Nothing;
                }
                let cur = self.player.current.or(self.selected).unwrap_or(0) as i32;
                let next = (cur + d).clamp(0, n - 1) as usize;
                self.selected = Some(next);
                self.play_selected()
            }
            Action::Select(i) => {
                if i >= self.len() {
                    return Changed::Nothing;
                }
                self.selected = Some(i);
                self.play_selected()
            }
            Action::Play => self.play_selected(),
            Action::Toggle => {
                if !self.player.toggle() {
                    return self.play_selected(); // nothing loaded yet
                }
                Changed::Full
            }
            Action::Seek(f) => {
                self.player.seek_frac(f);
                Changed::Progress
            }
            Action::Tick => {
                let switched = self.player.poll();
                if switched {
                    Changed::Full
                } else if self.player.playing {
                    Changed::Progress
                } else {
                    Changed::Nothing
                }
            }
        }
    }

    fn move_cursor(&mut self, d: i32) -> bool {
        let n = self.len() as i32;
        if n == 0 {
            return false;
        }
        let cur = self.selected.unwrap_or(0) as i32;
        let next = (cur + d).clamp(0, n - 1) as usize;
        if Some(next) == self.selected {
            return false;
        }
        self.selected = Some(next);
        true
    }

    fn play_selected(&mut self) -> Changed {
        let Some((i, path)) = self.selected_path() else { return Changed::Nothing };
        let path = path.clone();
        let ok = self.player.play(i, &path);
        let (_, len) = self.player.progress();
        println!("play {ok}: {path} ({len:.1}s)");
        Changed::Full
    }
}
