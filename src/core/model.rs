// model: the app's semantic state and the actions that change it. No pixels, no
// geometry, no OS. A UI projects this; it never owns it.

use crate::core::player::Player;

pub struct Model {
    pub tracks: Vec<String>,
    pub selected: Option<usize>, // list cursor
    pub player: Player,
}

/// Everything the app can be told to do, independent of input devices.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Select(usize),
    Step(i32), // prev/next relative to the playing track
    Move(i32), // move the list cursor only
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
        Self { tracks, selected, player: Player::new() }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
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
                    Changed::Full
                } else {
                    Changed::Nothing
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
