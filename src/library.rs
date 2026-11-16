// library: walk the music folder with the `ignore` crate, keep audio files only.

use std::path::Path;

const AUDIO: &[&str] = &[
    "m4a", "mp3", "flac", "wav", "aiff", "aif", "aifc", "ogg", "oga", "opus", "wma", "aac", "alac",
    "m4b", "caf",
];

/// Full paths of audio files under `root`, honouring .gitignore/.ignore files.
pub fn scan(root: &str) -> Vec<String> {
    let mut out = Vec::new();
    if !Path::new(root).is_dir() {
        return out;
    }
    for entry in ignore::WalkBuilder::new(root).build().flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let is_audio = ext.as_deref().is_some_and(|e| AUDIO.contains(&e));
        if is_audio {
            out.push(path.to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

/// Row label: file name without extension.
pub fn label(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
}
