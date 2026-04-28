use crate::downloader::DownloadFormat;
use crate::ytdlp::YtDlpChannel;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub download_dir: Option<PathBuf>,
    pub format: String,
    pub audio_quality: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_ytdlp_channel")]
    pub ytdlp_channel: String,
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_ytdlp_channel() -> String {
    YtDlpChannel::default().as_str().to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_dir: None,
            format: "mp3".to_string(),
            audio_quality: "320K".to_string(),
            language: "auto".to_string(),
            ytdlp_channel: default_ytdlp_channel(),
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust-yt")
            .join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("설정 폴더 생성 실패: {}", e))?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| format!("설정 직렬화 실패: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("설정 파일 저장 실패: {}", e))?;

        Ok(())
    }

    pub fn format_to_string(format: &DownloadFormat) -> String {
        match format {
            DownloadFormat::Mp3 => "mp3",
            DownloadFormat::Wav => "wav",
            DownloadFormat::M4a => "m4a",
            DownloadFormat::Flac => "flac",
            DownloadFormat::Mp4 => "mp4",
            DownloadFormat::Webm => "webm",
        }
        .to_string()
    }

    pub fn string_to_format(s: &str) -> DownloadFormat {
        match s {
            "wav" => DownloadFormat::Wav,
            "m4a" => DownloadFormat::M4a,
            "flac" => DownloadFormat::Flac,
            "mp4" => DownloadFormat::Mp4,
            "webm" => DownloadFormat::Webm,
            _ => DownloadFormat::Mp3,
        }
    }

    pub fn ytdlp_channel(&self) -> YtDlpChannel {
        YtDlpChannel::from_str(&self.ytdlp_channel)
    }
}
