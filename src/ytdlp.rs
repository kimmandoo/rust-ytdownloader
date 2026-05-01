use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type YtDlpResult<T> = Result<T, String>;

pub const YTDLP_SOCKET_TIMEOUT_SECS: &str = "30";
const CURRENTLY_BROKEN_MARKER: &str = " (CURRENTLY BROKEN)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportedSiteStatus {
    Stable,
    Experimental,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportedSite {
    pub name: String,
    pub status: SupportedSiteStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportedSites {
    pub featured: Vec<SupportedSite>,
    pub extractors: Vec<SupportedSite>,
}

pub fn bundled_deno_path() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-yt");

    #[cfg(target_os = "windows")]
    {
        app_dir.join("deno.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        app_dir.join("deno")
    }
}

pub fn js_runtime_args() -> Vec<String> {
    let deno_path = bundled_deno_path();
    if deno_path.exists() {
        js_runtime_args_for_deno_path(&deno_path)
    } else {
        Vec::new()
    }
}

fn js_runtime_args_for_deno_path(deno_path: &Path) -> Vec<String> {
    vec![
        "--js-runtimes".to_string(),
        format!("deno:{}", deno_path.to_string_lossy()),
    ]
}

pub fn configure_ytdlp_command(command: &mut Command) {
    command
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YtDlpUpdateTarget {
    Current,
    Stable,
    Nightly,
    Master,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum YtDlpChannel {
    #[default]
    Stable,
    Nightly,
    Master,
}

impl YtDlpChannel {
    pub const ALL: [Self; 3] = [Self::Stable, Self::Nightly, Self::Master];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Nightly => "nightly",
            Self::Master => "master",
        }
    }

    pub fn from_config_value(value: &str) -> Self {
        match value {
            "nightly" => Self::Nightly,
            "master" => Self::Master,
            _ => Self::Stable,
        }
    }

    fn update_target(self) -> YtDlpUpdateTarget {
        match self {
            Self::Stable => YtDlpUpdateTarget::Stable,
            Self::Nightly => YtDlpUpdateTarget::Nightly,
            Self::Master => YtDlpUpdateTarget::Master,
        }
    }
}

pub fn update_ytdlp_channel(ytdlp_path: &Path, channel: YtDlpChannel) -> YtDlpResult<String> {
    update_ytdlp_to(ytdlp_path, channel.update_target())
        .map(|msg| format!("{} channel active ({})", channel.as_str(), msg))
}

pub fn update_ytdlp_to(ytdlp_path: &Path, target: YtDlpUpdateTarget) -> YtDlpResult<String> {
    let mut cmd = Command::new(ytdlp_path);
    configure_ytdlp_command(&mut cmd);
    cmd.args(update_args(target));

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run yt-dlp updater: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = [stderr.trim(), stdout.trim()]
            .iter()
            .copied()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" | ");

        return Err(if detail.is_empty() {
            format!("yt-dlp updater exited with {}", output.status)
        } else {
            detail
        });
    }

    Ok(extract_update_status(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub fn update_args(target: YtDlpUpdateTarget) -> Vec<&'static str> {
    match target {
        YtDlpUpdateTarget::Current => vec!["-U"],
        YtDlpUpdateTarget::Stable => vec!["--update-to", "stable"],
        YtDlpUpdateTarget::Nightly => vec!["--update-to", "nightly"],
        YtDlpUpdateTarget::Master => vec!["--update-to", "master"],
    }
}

pub fn supported_sites(ytdlp_path: &Path) -> YtDlpResult<SupportedSites> {
    Ok(SupportedSites {
        featured: featured_supported_sites(),
        extractors: list_supported_extractors(ytdlp_path)?,
    })
}

pub fn featured_supported_sites() -> Vec<SupportedSite> {
    [
        ("YouTube", SupportedSiteStatus::Stable),
        ("YouTube Music", SupportedSiteStatus::Stable),
        ("TikTok", SupportedSiteStatus::Experimental),
        ("SoundCloud", SupportedSiteStatus::Experimental),
        ("Vimeo", SupportedSiteStatus::Experimental),
        ("X/Twitter", SupportedSiteStatus::Experimental),
        ("Instagram", SupportedSiteStatus::Experimental),
        ("Twitch", SupportedSiteStatus::Experimental),
        ("Facebook", SupportedSiteStatus::Experimental),
        ("Bilibili", SupportedSiteStatus::Experimental),
        ("Niconico", SupportedSiteStatus::Experimental),
    ]
    .into_iter()
    .map(|(name, status)| SupportedSite {
        name: name.to_string(),
        status,
    })
    .collect()
}

pub fn list_supported_extractors(ytdlp_path: &Path) -> YtDlpResult<Vec<SupportedSite>> {
    let mut cmd = Command::new(ytdlp_path);
    configure_ytdlp_command(&mut cmd);
    cmd.arg("--list-extractors");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to list yt-dlp extractors: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(if stderr.trim().is_empty() {
            format!("yt-dlp extractor list exited with {}", output.status)
        } else {
            stderr.trim().to_string()
        });
    }

    Ok(parse_extractor_list(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn parse_extractor_list(stdout: &str) -> Vec<SupportedSite> {
    stdout.lines().filter_map(parse_extractor_line).collect()
}

pub(crate) fn parse_extractor_line(line: &str) -> Option<SupportedSite> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(name) = line.strip_suffix(CURRENTLY_BROKEN_MARKER) {
        return Some(SupportedSite {
            name: name.trim().to_string(),
            status: SupportedSiteStatus::Broken,
        });
    }

    Some(SupportedSite {
        name: line.to_string(),
        status: SupportedSiteStatus::Experimental,
    })
}

pub(crate) fn source_display_name(
    extractor: Option<&str>,
    extractor_key: Option<&str>,
    url: &str,
) -> Option<String> {
    let raw = extractor
        .filter(|value| !value.trim().is_empty())
        .or_else(|| extractor_key.filter(|value| !value.trim().is_empty()));

    if let Some(raw) = raw
        && !raw.eq_ignore_ascii_case("generic")
    {
        return Some(normalize_source_name(raw));
    }

    source_display_name_from_url(url)
}

fn normalize_source_name(raw: &str) -> String {
    let raw = raw.trim();
    let lowered = raw.to_ascii_lowercase();

    if lowered.contains("youtube") || lowered == "youtu" {
        "YouTube".to_string()
    } else if lowered.contains("tiktok") {
        "TikTok".to_string()
    } else if lowered.contains("soundcloud") {
        "SoundCloud".to_string()
    } else if lowered.contains("vimeo") {
        "Vimeo".to_string()
    } else if lowered.contains("twitter") || lowered == "x" {
        "X/Twitter".to_string()
    } else if lowered.contains("instagram") {
        "Instagram".to_string()
    } else if lowered.contains("twitch") {
        "Twitch".to_string()
    } else if lowered.contains("facebook") {
        "Facebook".to_string()
    } else if lowered.contains("bilibili") {
        "Bilibili".to_string()
    } else if lowered.contains("niconico") || lowered.contains("nicovideo") {
        "Niconico".to_string()
    } else {
        raw.to_string()
    }
}

fn source_display_name_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches("www.")
        .to_ascii_lowercase();

    if host.is_empty() {
        return None;
    }

    Some(match host.as_str() {
        "youtu.be" | "youtube.com" | "music.youtube.com" => "YouTube".to_string(),
        "tiktok.com" => "TikTok".to_string(),
        "soundcloud.com" => "SoundCloud".to_string(),
        "vimeo.com" => "Vimeo".to_string(),
        "x.com" | "twitter.com" => "X/Twitter".to_string(),
        "instagram.com" => "Instagram".to_string(),
        "twitch.tv" => "Twitch".to_string(),
        "facebook.com" | "fb.watch" => "Facebook".to_string(),
        "bilibili.com" => "Bilibili".to_string(),
        "nicovideo.jp" | "niconico.com" => "Niconico".to_string(),
        _ => host,
    })
}

pub fn should_retry_after_channel_update(url: &str, error: &str) -> bool {
    if !is_youtube_url(url) {
        return false;
    }

    let error = error.to_ascii_lowercase();
    [
        "unable to extract",
        "nsig",
        "n challenge",
        "signature",
        "http error 403",
        "forbidden",
        "player response",
        "no video formats",
    ]
    .iter()
    .any(|needle| error.contains(needle))
}

pub(crate) fn is_youtube_url(url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    url.contains("youtube.com/") || url.contains("youtu.be/")
}

fn extract_update_status(stdout: &str) -> String {
    stdout
        .lines()
        .find(|line| {
            line.contains("up to date")
                || line.contains("Updated")
                || line.contains("Current version")
        })
        .unwrap_or("yt-dlp update check completed")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_channel_update_uses_regular_update_flag() {
        assert_eq!(update_args(YtDlpUpdateTarget::Current), vec!["-U"]);
    }

    #[test]
    fn nightly_update_target_switches_release_channel() {
        assert_eq!(
            update_args(YtDlpUpdateTarget::Nightly),
            vec!["--update-to", "nightly"]
        );
    }

    #[test]
    fn youtube_extractor_failures_are_channel_update_retry_candidates() {
        let error = "ERROR: [youtube] abc: Unable to extract nsig function code";

        assert!(should_retry_after_channel_update(
            "https://www.youtube.com/watch?v=abc",
            error
        ));
    }

    #[test]
    fn non_youtube_failures_do_not_trigger_channel_switch() {
        assert!(!should_retry_after_channel_update(
            "https://example.com/video.mp4",
            "HTTP Error 403: Forbidden"
        ));
    }

    #[test]
    fn stable_update_target_supports_rollback() {
        assert_eq!(
            update_args(YtDlpUpdateTarget::Stable),
            vec!["--update-to", "stable"]
        );
    }

    #[test]
    fn default_user_channel_is_stable() {
        assert_eq!(YtDlpChannel::default(), YtDlpChannel::Stable);
    }

    #[test]
    fn unknown_user_channel_falls_back_to_stable() {
        assert_eq!(
            YtDlpChannel::from_config_value("bad-channel"),
            YtDlpChannel::Stable
        );
    }

    #[test]
    fn user_channel_round_trips_as_config_string() {
        for channel in YtDlpChannel::ALL {
            assert_eq!(YtDlpChannel::from_config_value(channel.as_str()), channel);
        }
    }

    #[test]
    fn js_runtime_args_pin_bundled_deno_path() {
        let args = js_runtime_args_for_deno_path(Path::new("C:/tools/deno.exe"));

        assert_eq!(
            args,
            vec![
                "--js-runtimes".to_string(),
                "deno:C:/tools/deno.exe".to_string()
            ]
        );
    }

    #[test]
    fn ytdlp_commands_force_utf8_python_stdio() {
        let mut command = Command::new("yt-dlp");

        configure_ytdlp_command(&mut command);

        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(
            envs.get("PYTHONIOENCODING").and_then(|value| value.as_deref()),
            Some("utf-8")
        );
        assert_eq!(
            envs.get("PYTHONUTF8").and_then(|value| value.as_deref()),
            Some("1")
        );
    }

    #[test]
    fn parses_regular_extractor_as_experimental_site() {
        assert_eq!(
            parse_extractor_line("SoundCloud"),
            Some(SupportedSite {
                name: "SoundCloud".to_string(),
                status: SupportedSiteStatus::Experimental,
            })
        );
    }

    #[test]
    fn parses_currently_broken_extractor_status() {
        assert_eq!(
            parse_extractor_line("247sports (CURRENTLY BROKEN)"),
            Some(SupportedSite {
                name: "247sports".to_string(),
                status: SupportedSiteStatus::Broken,
            })
        );
    }

    #[test]
    fn featured_sites_put_stable_youtube_first() {
        let sites = featured_supported_sites();

        assert_eq!(sites[0].name, "YouTube");
        assert_eq!(sites[0].status, SupportedSiteStatus::Stable);
        assert!(sites.iter().any(|site| site.name == "TikTok"));
    }
}
