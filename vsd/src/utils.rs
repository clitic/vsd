use std::{env, path::PathBuf};

pub fn find_ffmpeg() -> Option<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::current_dir().ok() {
        paths.push(path);
    }
    if let Some(path) = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
    {
        paths.push(path);
    }
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    #[cfg(target_os = "windows")]
    let bin = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    let bin = "ffmpeg";
    paths.into_iter().find(|path| path.join(bin).exists())
}
