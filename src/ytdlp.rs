use std::path::Path;
use std::process::Command;

pub type YtDlpResult<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YtDlpUpdateTarget {
    Current,
    Stable,
    Nightly,
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YtDlpChannel {
    Stable,
    Nightly,
    Master,
}

impl Default for YtDlpChannel {
    fn default() -> Self {
        Self::Stable
    }
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

    pub fn from_str(value: &str) -> Self {
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

fn is_youtube_url(url: &str) -> bool {
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
        assert_eq!(YtDlpChannel::from_str("bad-channel"), YtDlpChannel::Stable);
    }

    #[test]
    fn user_channel_round_trips_as_config_string() {
        for channel in YtDlpChannel::ALL {
            assert_eq!(YtDlpChannel::from_str(channel.as_str()), channel);
        }
    }
}
