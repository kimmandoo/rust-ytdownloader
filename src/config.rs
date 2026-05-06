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
    #[serde(default = "default_ytdlp_cookie_browser")]
    pub ytdlp_cookie_browser: String,
    #[serde(default)]
    pub ytdlp_cookie_file: Option<PathBuf>,
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_ytdlp_channel() -> String {
    YtDlpChannel::default().as_str().to_string()
}

fn default_ytdlp_cookie_browser() -> String {
    crate::ytdlp::YtDlpCookieBrowser::default()
        .as_config_value()
        .to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_dir: None,
            format: "mp3".to_string(),
            audio_quality: "320K".to_string(),
            language: "auto".to_string(),
            ytdlp_channel: default_ytdlp_channel(),
            ytdlp_cookie_browser: default_ytdlp_cookie_browser(),
            ytdlp_cookie_file: None,
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
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(config) = toml::from_str(&content)
        {
            return config;
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
        YtDlpChannel::from_config_value(&self.ytdlp_channel)
    }

    pub fn ytdlp_cookie_browser(&self) -> crate::ytdlp::YtDlpCookieBrowser {
        crate::ytdlp::YtDlpCookieBrowser::from_config_value(&self.ytdlp_cookie_browser)
    }

    pub fn ytdlp_cookie_source(&self) -> crate::ytdlp::YtDlpCookieSource {
        crate::ytdlp::YtDlpCookieSource::from_settings(
            self.ytdlp_cookie_browser(),
            self.ytdlp_cookie_file.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ytdlp::YtDlpCookieBrowser;

    #[test]
    fn default_config_disables_browser_cookies() {
        assert_eq!(
            AppConfig::default().ytdlp_cookie_browser(),
            YtDlpCookieBrowser::None
        );
    }

    #[test]
    fn cookie_browser_config_parses_known_browser() {
        let config = AppConfig {
            ytdlp_cookie_browser: "chrome".to_string(),
            ..AppConfig::default()
        };

        assert_eq!(config.ytdlp_cookie_browser(), YtDlpCookieBrowser::Chrome);
    }

    #[test]
    fn cookie_file_config_overrides_browser_cookie_source() {
        let config = AppConfig {
            ytdlp_cookie_browser: "chrome".to_string(),
            ytdlp_cookie_file: Some(PathBuf::from("C:/cookies/youtube.txt")),
            ..AppConfig::default()
        };

        assert_eq!(
            config.ytdlp_cookie_source(),
            crate::ytdlp::YtDlpCookieSource::File(PathBuf::from("C:/cookies/youtube.txt"))
        );
    }
}
