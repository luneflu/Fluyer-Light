// player: transport state over the `fluyer-audio` backend.

use fluyer_audio as audio;

pub struct Player {
    pub current: Option<usize>, // index of the loaded track, not the list cursor
    pub playing: bool,
}

impl Player {
    pub fn new() -> Self {
        Self { current: None, playing: false }
    }

    pub fn init(&self) -> bool {
        audio::init()
    }

    pub fn play(&mut self, index: usize, path: &str) -> bool {
        let ok = audio::play(path);
        self.current = Some(index);
        self.playing = audio::is_playing();
        ok
    }

    /// Resume/pause; returns false when nothing is loaded yet.
    pub fn toggle(&mut self) -> bool {
        if self.progress().1 == 0.0 {
            return false;
        }
        audio::toggle();
        self.playing = audio::is_playing();
        true
    }

    pub fn seek_frac(&mut self, f: f64) {
        let (_, len) = self.progress();
        audio::seek(f * len);
    }

    /// (position, length) in seconds.
    pub fn progress(&self) -> (f64, f64) {
        audio::progress()
    }

    /// Poll the backend; returns true when the drawn state changed.
    pub fn poll(&mut self) -> bool {
        let playing = audio::is_playing();
        if playing == self.playing {
            return false;
        }
        self.playing = playing;
        true
    }

    pub fn quit(&self) {
        audio::quit()
    }
}
