use serde::{Deserialize, Serialize};
use std::process::Command;

/// 플레이리스트 또는 단일 영상 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub title: String,
    pub entries: Vec<VideoEntry>,
    pub is_playlist: bool,
}

/// 개별 영상 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: Option<String>, // [NEW] 썸네일 URL
    pub duration: Option<f64>,
    pub duration_string: Option<String>,
    #[serde(default)]
    pub selected: bool,
}

impl VideoEntry {
    pub fn format_duration(&self) -> String {
        if let Some(dur_str) = &self.duration_string {
            dur_str.clone()
        } else if let Some(dur) = self.duration {
            let mins = (dur / 60.0) as u32;
            let secs = (dur % 60.0) as u32;
            format!("{}:{:02}", mins, secs)
        } else {
            "??:??".to_string()
        }
    }
}

/// yt-dlp JSON 응답 파싱용 구조체
#[derive(Debug, Deserialize)]
struct YtDlpResponse {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    webpage_url: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>, // [NEW]
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    duration_string: Option<String>,
    #[serde(default)]
    entries: Option<Vec<YtDlpEntry>>,
    #[serde(rename = "_type", default)]
    response_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YtDlpEntry {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>, // [NEW]
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    duration_string: Option<String>,
}

/// yt-dlp 경로 가져오기
pub fn get_ytdlp_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let app_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("rust-yt");
        app_dir.join("yt-dlp.exe")
    }
    #[cfg(target_os = "macos")]
    {
        std::path::PathBuf::from("yt-dlp")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home"));
        let pipx_path = std::path::PathBuf::from(format!("{}/.local/bin/yt-dlp", home));
        if pipx_path.exists() {
            return pipx_path;
        }
        std::path::PathBuf::from("yt-dlp")
    }
}

/// URL에서 플레이리스트/영상 정보 가져오기
pub fn fetch_playlist_info(url: &str) -> Result<PlaylistInfo, String> {
    let ytdlp = get_ytdlp_path();
    
    let mut command = Command::new(&ytdlp);
    command.args([
            "--flat-playlist",
            "-J",
            "--no-warnings",
            url,
        ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output()
        .map_err(|e| format!("yt-dlp 실행 실패: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("영상 정보를 가져올 수 없습니다: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let response: YtDlpResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON 파싱 실패: {}", e))?;

    let is_playlist = response.response_type.as_deref() == Some("playlist");
    
    if is_playlist {
        // 플레이리스트
        let entries = response.entries.unwrap_or_default()
            .into_iter()
            .filter_map(|e| {
                let id = e.id?;
                Some(VideoEntry {
                    id: id.clone(),
                    title: e.title.unwrap_or_else(|| "제목 없음".to_string()),
                    url: e.url.unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id)),
                    thumbnail: e.thumbnail,
                    duration: e.duration,
                    duration_string: e.duration_string,
                    selected: true,
                })
            })
            .collect();

        Ok(PlaylistInfo {
            title: response.title.unwrap_or_else(|| "플레이리스트".to_string()),
            entries,
            is_playlist: true,
        })
    } else {
        // 단일 영상
        let entry = VideoEntry {
            id: response.id.unwrap_or_default(),
            title: response.title.clone().unwrap_or_else(|| "제목 없음".to_string()),
            url: response.webpage_url.unwrap_or_else(|| url.to_string()),
            thumbnail: response.thumbnail,
            duration: response.duration,
            duration_string: response.duration_string,
            selected: true,
        };

        Ok(PlaylistInfo {
            title: response.title.unwrap_or_else(|| "영상".to_string()),
            entries: vec![entry],
            is_playlist: false,
        })
    }
}
