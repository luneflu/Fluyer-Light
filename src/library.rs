// library: plain std::fs walk, audio files only. No gitignore support (drop of the `ignore` crate).

use std::path::Path;

const AUDIO: &[&str] = &[
    "m4a", "mp3", "flac", "wav", "aiff", "aif", "aifc", "ogg", "oga", "opus", "wma", "aac", "alac",
    "m4b", "caf",
];

pub fn scan(root: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk(Path::new(root), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            walk(&path, out);
        } else if ft.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if ext.as_deref().is_some_and(|e| AUDIO.contains(&e)) {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
}

pub fn label(path: &str) -> &str {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path)
}
