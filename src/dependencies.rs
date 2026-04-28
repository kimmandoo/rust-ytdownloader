use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn app_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-yt")
}

pub fn ytdlp_path() -> PathBuf {
    app_dir().join(ytdlp_file_name())
}

pub fn ffmpeg_path() -> PathBuf {
    app_dir().join(ffmpeg_file_name())
}

pub fn dependency_path_env() -> String {
    let app_dir = app_dir();
    let current_path = std::env::var_os("PATH").unwrap_or_default();

    let mut paths = Vec::new();
    paths.push(app_dir);
    paths.extend(std::env::split_paths(&current_path));

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/opt/homebrew/bin"));
        paths.push(PathBuf::from("/usr/local/bin"));
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join(".cargo/bin"));
        }
    }

    join_paths_or_current(paths, current_path)
}

pub fn ytdlp_file_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "yt-dlp.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "yt-dlp"
    }
}

pub fn ffmpeg_file_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "ffmpeg.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "ffmpeg"
    }
}

fn join_paths_or_current(paths: Vec<PathBuf>, current_path: OsString) -> String {
    std::env::join_paths(paths)
        .unwrap_or(current_path)
        .to_string_lossy()
        .to_string()
}

pub fn fallback_command(path: &Path, command: &'static str) -> PathBuf {
    if path.exists() {
        path.to_path_buf()
    } else {
        PathBuf::from(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_command_prefers_managed_binary_when_it_exists() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

        assert_eq!(fallback_command(&path, "yt-dlp"), path);
    }

    #[test]
    fn fallback_command_uses_system_command_when_managed_binary_is_missing() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("definitely-missing-binary");

        assert_eq!(fallback_command(&path, "yt-dlp"), PathBuf::from("yt-dlp"));
    }
}
