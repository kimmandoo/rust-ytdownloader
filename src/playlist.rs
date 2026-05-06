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
    #[serde(default)]
    pub source: Option<String>,
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
    extractor: Option<String>,
    #[serde(default)]
    extractor_key: Option<String>,
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
    webpage_url: Option<String>,
    #[serde(default)]
    extractor: Option<String>,
    #[serde(default)]
    extractor_key: Option<String>,
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
    fetch_playlist_info_with_channel(url, crate::ytdlp::YtDlpChannel::default())
}

pub fn fetch_playlist_info_with_channel(
    url: &str,
    ytdlp_channel: crate::ytdlp::YtDlpChannel,
) -> Result<PlaylistInfo, String> {
    fetch_playlist_info_with_options(url, ytdlp_channel, crate::ytdlp::YtDlpCookieBrowser::None)
}

pub fn fetch_playlist_info_with_options(
    url: &str,
    ytdlp_channel: crate::ytdlp::YtDlpChannel,
    cookie_browser: crate::ytdlp::YtDlpCookieBrowser,
) -> Result<PlaylistInfo, String> {
    fetch_playlist_info_with_retry(url, ytdlp_channel, cookie_browser, true)
}

fn fetch_playlist_info_with_retry(
    url: &str,
    ytdlp_channel: crate::ytdlp::YtDlpChannel,
    cookie_browser: crate::ytdlp::YtDlpCookieBrowser,
    allow_nightly_retry: bool,
) -> Result<PlaylistInfo, String> {
    let ytdlp = get_ytdlp_path();

    let mut command = Command::new(&ytdlp);
    crate::ytdlp::configure_ytdlp_command(&mut command);
    command.args(playlist_info_args(url, cookie_browser));

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .map_err(|e| format!("yt-dlp 실행 실패: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if should_retry_without_browser_cookies(cookie_browser, &stderr) {
            return fetch_playlist_info_with_retry(
                url,
                ytdlp_channel,
                crate::ytdlp::YtDlpCookieBrowser::None,
                allow_nightly_retry,
            )
            .map_err(|fallback_error| {
                format!(
                    "{} Fallback without browser cookies also failed: {}",
                    crate::ytdlp::browser_cookie_error_message(cookie_browser),
                    fallback_error
                )
            });
        }
        if allow_nightly_retry && crate::ytdlp::should_retry_after_channel_update(url, &stderr) {
            match crate::ytdlp::update_ytdlp_channel(&ytdlp, ytdlp_channel) {
                Ok(_) => {
                    return fetch_playlist_info_with_retry(
                        url,
                        ytdlp_channel,
                        cookie_browser,
                        false,
                    );
                }
                Err(update_error) => {
                    return Err(format!(
                        "Failed to fetch video info: {} | {} update failed: {}",
                        stderr,
                        ytdlp_channel.as_str(),
                        update_error
                    ));
                }
            }
        }
        return Err(format_playlist_fetch_error(&stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let response: YtDlpResponse =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON 파싱 실패: {}", e))?;

    playlist_info_from_response(response, url)
}

fn playlist_info_from_response(
    response: YtDlpResponse,
    source_url: &str,
) -> Result<PlaylistInfo, String> {
    let is_playlist = response.response_type.as_deref() == Some("playlist");
    let parent_source = crate::ytdlp::source_display_name(
        response.extractor.as_deref(),
        response.extractor_key.as_deref(),
        source_url,
    );

    if is_playlist {
        // 플레이리스트
        let entries = response
            .entries
            .unwrap_or_default()
            .into_iter()
            .filter_map(|e| {
                let id = e.id?;
                let url = resolve_entry_url(e.webpage_url, e.url, &id, source_url)?;
                let source = crate::ytdlp::source_display_name(
                    e.extractor.as_deref(),
                    e.extractor_key.as_deref(),
                    &url,
                )
                .or_else(|| parent_source.clone());
                Some(VideoEntry {
                    id: id.clone(),
                    title: e.title.unwrap_or_else(|| "제목 없음".to_string()),
                    url,
                    source,
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
        let entry_url = response
            .webpage_url
            .clone()
            .unwrap_or_else(|| source_url.to_string());
        let source = crate::ytdlp::source_display_name(
            response.extractor.as_deref(),
            response.extractor_key.as_deref(),
            &entry_url,
        );
        let entry = VideoEntry {
            id: response.id.unwrap_or_default(),
            title: response
                .title
                .clone()
                .unwrap_or_else(|| "제목 없음".to_string()),
            url: entry_url,
            source,
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

fn resolve_entry_url(
    webpage_url: Option<String>,
    url: Option<String>,
    id: &str,
    source_url: &str,
) -> Option<String> {
    if let Some(webpage_url) = webpage_url.filter(|value| is_absolute_http_url(value)) {
        return Some(webpage_url);
    }

    if let Some(url) = url.filter(|value| is_absolute_http_url(value)) {
        return Some(url);
    }

    if crate::ytdlp::is_youtube_url(source_url) {
        return Some(format!("https://www.youtube.com/watch?v={}", id));
    }

    None
}

fn should_retry_without_browser_cookies(
    cookie_browser: crate::ytdlp::YtDlpCookieBrowser,
    stderr: &str,
) -> bool {
    cookie_browser != crate::ytdlp::YtDlpCookieBrowser::None
        && crate::ytdlp::is_browser_cookie_extraction_error(stderr)
}

fn is_absolute_http_url(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("https://") || value.starts_with("http://")
}

fn playlist_info_args(url: &str, cookie_browser: crate::ytdlp::YtDlpCookieBrowser) -> Vec<String> {
    let mut args = vec![
        "--flat-playlist".to_string(),
        "-J".to_string(),
        "--no-warnings".to_string(),
        "--socket-timeout".to_string(),
        crate::ytdlp::YTDLP_SOCKET_TIMEOUT_SECS.to_string(),
    ];
    args.extend(crate::ytdlp::cookie_browser_args(cookie_browser));
    args.extend(crate::ytdlp::js_runtime_args());
    args.push(url.to_string());
    args
}

fn format_playlist_fetch_error(stderr: &str) -> String {
    let stderr = stderr.trim();
    let lowered = stderr.to_ascii_lowercase();

    if lowered.contains("connectionreseterror(10054")
        || lowered.contains("connection reset by peer")
        || lowered.contains("connection aborted")
    {
        return "영상 정보를 가져올 수 없습니다: 원격 사이트가 연결을 강제로 종료했습니다. 잠시 후 다시 시도하거나 링크 접근 가능 여부, VPN/방화벽, 네트워크 상태를 확인해 주세요."
            .to_string();
    }

    if stderr.is_empty() {
        "영상 정보를 가져올 수 없습니다.".to_string()
    } else {
        format!("영상 정보를 가져올 수 없습니다: {}", stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_playlist_entries_without_urls_keep_youtube_fallback() {
        let response = YtDlpResponse {
            title: Some("Mix".to_string()),
            id: None,
            webpage_url: None,
            extractor: None,
            extractor_key: None,
            thumbnail: None,
            duration: None,
            duration_string: None,
            entries: Some(vec![YtDlpEntry {
                id: Some("abc123".to_string()),
                title: Some("Track".to_string()),
                url: None,
                webpage_url: None,
                extractor: None,
                extractor_key: None,
                thumbnail: None,
                duration: None,
                duration_string: None,
            }]),
            response_type: Some("playlist".to_string()),
        };

        let info =
            playlist_info_from_response(response, "https://www.youtube.com/playlist?list=example")
                .unwrap();

        assert_eq!(
            info.entries[0].url,
            "https://www.youtube.com/watch?v=abc123"
        );
    }

    #[test]
    fn generic_playlist_entries_without_urls_are_skipped() {
        let response = YtDlpResponse {
            title: Some("Creator feed".to_string()),
            id: None,
            webpage_url: None,
            extractor: None,
            extractor_key: None,
            thumbnail: None,
            duration: None,
            duration_string: None,
            entries: Some(vec![YtDlpEntry {
                id: Some("post-1".to_string()),
                title: Some("Clip".to_string()),
                url: None,
                webpage_url: None,
                extractor: None,
                extractor_key: None,
                thumbnail: None,
                duration: None,
                duration_string: None,
            }]),
            response_type: Some("playlist".to_string()),
        };

        let info = playlist_info_from_response(response, "https://example.com/creator").unwrap();

        assert!(info.entries.is_empty());
    }

    #[test]
    fn generic_playlist_entries_prefer_webpage_urls() {
        let response = YtDlpResponse {
            title: Some("Creator feed".to_string()),
            id: None,
            webpage_url: None,
            extractor: None,
            extractor_key: None,
            thumbnail: None,
            duration: None,
            duration_string: None,
            entries: Some(vec![YtDlpEntry {
                id: Some("post-1".to_string()),
                title: Some("Clip".to_string()),
                url: Some("post-1".to_string()),
                webpage_url: Some("https://example.com/watch/post-1".to_string()),
                extractor: None,
                extractor_key: None,
                thumbnail: None,
                duration: None,
                duration_string: None,
            }]),
            response_type: Some("playlist".to_string()),
        };

        let info = playlist_info_from_response(response, "https://example.com/creator").unwrap();

        assert_eq!(info.entries[0].url, "https://example.com/watch/post-1");
    }

    #[test]
    fn single_video_includes_extractor_source() {
        let response = YtDlpResponse {
            title: Some("Short clip".to_string()),
            id: Some("clip-1".to_string()),
            webpage_url: Some("https://www.tiktok.com/@spull/video/1".to_string()),
            extractor: Some("TikTok".to_string()),
            extractor_key: Some("TikTok".to_string()),
            thumbnail: None,
            duration: None,
            duration_string: None,
            entries: None,
            response_type: None,
        };

        let info =
            playlist_info_from_response(response, "https://www.tiktok.com/@spull/video/1").unwrap();

        assert_eq!(info.entries[0].source.as_deref(), Some("TikTok"));
    }

    #[test]
    fn playlist_entry_prefers_entry_source_over_parent_source() {
        let response = YtDlpResponse {
            title: Some("Mixed feed".to_string()),
            id: None,
            webpage_url: None,
            extractor: Some("Generic".to_string()),
            extractor_key: Some("Generic".to_string()),
            thumbnail: None,
            duration: None,
            duration_string: None,
            entries: Some(vec![YtDlpEntry {
                id: Some("track-1".to_string()),
                title: Some("Track".to_string()),
                url: Some("https://soundcloud.com/spull/track-1".to_string()),
                webpage_url: Some("https://soundcloud.com/spull/track-1".to_string()),
                extractor: Some("SoundCloud".to_string()),
                extractor_key: Some("SoundCloud".to_string()),
                thumbnail: None,
                duration: None,
                duration_string: None,
            }]),
            response_type: Some("playlist".to_string()),
        };

        let info = playlist_info_from_response(response, "https://example.com/feed").unwrap();

        assert_eq!(info.entries[0].source.as_deref(), Some("SoundCloud"));
    }

    #[test]
    fn playlist_info_uses_30_second_socket_timeout() {
        let args = playlist_info_args(
            "https://youtu.be/example",
            crate::ytdlp::YtDlpCookieBrowser::None,
        );

        assert!(
            args.windows(2)
                .any(|pair| pair == ["--socket-timeout", "30"])
        );
    }

    #[test]
    fn playlist_info_can_load_browser_cookies() {
        let args = playlist_info_args(
            "https://youtu.be/age-gated",
            crate::ytdlp::YtDlpCookieBrowser::Chrome,
        );

        assert!(
            args.windows(2)
                .any(|pair| pair == ["--cookies-from-browser", "chrome"])
        );
    }

    #[test]
    fn browser_cookie_database_copy_errors_retry_without_browser_cookies() {
        let stderr = "ERROR: Could not copy Chrome cookie database.";

        assert!(should_retry_without_browser_cookies(
            crate::ytdlp::YtDlpCookieBrowser::Chrome,
            stderr
        ));
    }

    #[test]
    fn browser_cookie_dpapi_errors_retry_without_browser_cookies() {
        let stderr = "ERROR: Failed to decrypt with DPAPI.";

        assert!(should_retry_without_browser_cookies(
            crate::ytdlp::YtDlpCookieBrowser::Chrome,
            stderr
        ));
    }

    #[test]
    fn browser_cookie_database_copy_errors_do_not_retry_when_cookies_are_disabled() {
        let stderr = "ERROR: Could not copy Chrome cookie database.";

        assert!(!should_retry_without_browser_cookies(
            crate::ytdlp::YtDlpCookieBrowser::None,
            stderr
        ));
    }

    #[test]
    fn connection_reset_errors_are_summarized_for_users() {
        let stderr = "ERROR: [example] abc: Unable to download webpage: ('Connection aborted.', ConnectionResetError(10054, '���� ������ ���� ȣ��Ʈ�� ���� ������ ������ϴ�', None, 10054, None))";

        let message = format_playlist_fetch_error(stderr);

        assert!(message.contains("연결을 강제로 종료"));
        assert!(!message.contains("����"));
    }
}
