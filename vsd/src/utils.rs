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
    paths.into_iter().map(|x| x.join(bin)).find(|x| x.exists())
}

pub fn gen_id(base_url: &str, uri: &str) -> String {
    blake3::hash(format!("{}+{}", base_url, uri).as_bytes()).to_hex()[..7].to_owned()
}
