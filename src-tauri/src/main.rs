use rust_yt::config::AppConfig;
use rust_yt::downloader::{download_video, DownloadConfig, DownloadFormat, DownloadStatus};
use rust_yt::initializer::{init_dependencies, InitStatus};
use rust_yt::playlist::{fetch_playlist_info_with_channel, PlaylistInfo, VideoEntry};
use rust_yt::ytdlp::YtDlpChannel;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
struct RuntimeState {
    stop_tx: Mutex<Option<std::sync::mpsc::Sender<()>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsDto {
    download_dir: Option<String>,
    format: String,
    audio_quality: String,
    ytdlp_channel: String,
}

#[derive(Debug, Clone, Serialize)]
struct InitEvent {
    kind: &'static str,
    message: String,
    percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadEvent {
    kind: &'static str,
    current: usize,
    total: usize,
    percent: f64,
    title: String,
    message: String,
}

fn main() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            choose_folder,
            initialize,
            analyze_url,
            start_download,
            stop_download,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}

#[tauri::command]
fn get_settings() -> SettingsDto {
    SettingsDto::from_config(AppConfig::load())
}

#[tauri::command]
fn save_settings(settings: SettingsDto) -> Result<SettingsDto, String> {
    let config = settings.to_config();
    config.save()?;
    Ok(SettingsDto::from_config(config))
}

#[tauri::command]
fn choose_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn initialize(app: AppHandle, ytdlp_channel: String) -> Result<(), String> {
    let selected_channel = YtDlpChannel::from_str(&ytdlp_channel);
    thread::spawn(move || {
        let (tx, rx) = channel();
        thread::spawn(move || init_dependencies(tx, selected_channel));

        while let Ok(status) = rx.recv() {
            let event = match status {
                InitStatus::Starting(message) => InitEvent {
                    kind: "starting",
                    message,
                    percent: None,
                },
                InitStatus::Downloading(percent, file) => InitEvent {
                    kind: "downloading",
                    message: format!("{} 다운로드 중", file),
                    percent: Some(percent),
                },
                InitStatus::Extracting(file) => InitEvent {
                    kind: "extracting",
                    message: format!("{} 압축 해제 중", file),
                    percent: None,
                },
                InitStatus::Completed => InitEvent {
                    kind: "completed",
                    message: "준비 완료".to_string(),
                    percent: Some(100.0),
                },
                InitStatus::Failed(message) => InitEvent {
                    kind: "failed",
                    message,
                    percent: None,
                },
            };

            let done = matches!(event.kind, "completed" | "failed");
            let _ = app.emit("init-progress", event);
            if done {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn analyze_url(url: String, ytdlp_channel: String) -> Result<PlaylistInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fetch_playlist_info_with_channel(&url, YtDlpChannel::from_str(&ytdlp_channel))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn start_download(
    app: AppHandle,
    state: State<RuntimeState>,
    entries: Vec<VideoEntry>,
    format: String,
    output_dir: String,
) -> Result<(), String> {
    let selected_entries: Vec<VideoEntry> =
        entries.into_iter().filter(|entry| entry.selected).collect();
    if selected_entries.is_empty() {
        return Err("다운로드할 영상이 없습니다.".to_string());
    }

    let output_dir = PathBuf::from(output_dir);
    if !output_dir.exists() {
        return Err("저장 폴더가 존재하지 않습니다.".to_string());
    }

    let (stop_tx, stop_rx) = channel::<()>();
    *state
        .stop_tx
        .lock()
        .map_err(|_| "중지 상태를 잠글 수 없습니다.")? = Some(stop_tx);

    thread::spawn(move || {
        let total = selected_entries.len();
        let mut stop_rx = Some(stop_rx);

        for (idx, video) in selected_entries.into_iter().enumerate() {
            let current = idx + 1;
            let (status_tx, status_rx) = channel();
            let (_, fallback_stop_rx) = channel();
            let video_title = video.title.clone();
            let config = DownloadConfig {
                url: video.url.clone(),
                format: string_to_format(&format),
                audio_quality: "320K".to_string(),
                output_dir: output_dir.clone(),
            };
            let receiver = stop_rx.take().unwrap_or(fallback_stop_rx);

            thread::spawn(move || {
                download_video(config, video_title, status_tx, receiver);
            });

            let mut failed_or_stopped = false;
            while let Ok(status) = status_rx.recv() {
                let event = download_event_from_status(status, current, total, &video.title);
                failed_or_stopped = matches!(event.kind, "failed" | "stopped");
                let _ = app.emit("download-progress", event.clone());
                if matches!(event.kind, "completed" | "failed" | "stopped") {
                    break;
                }
            }

            if failed_or_stopped {
                return;
            }
        }

        let _ = app.emit(
            "download-progress",
            DownloadEvent {
                kind: "all_completed",
                current: total,
                total,
                percent: 100.0,
                title: "완료".to_string(),
                message: "모든 작업이 완료되었습니다.".to_string(),
            },
        );
    });

    Ok(())
}

#[tauri::command]
fn stop_download(state: State<RuntimeState>) -> Result<(), String> {
    if let Some(tx) = state
        .stop_tx
        .lock()
        .map_err(|_| "중지 상태를 잠글 수 없습니다.")?
        .take()
    {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("폴더가 존재하지 않습니다.".to_string());
    }

    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(path).spawn();

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(path).spawn();

    Ok(())
}

impl SettingsDto {
    fn from_config(config: AppConfig) -> Self {
        Self {
            download_dir: config
                .download_dir
                .map(|path| path.to_string_lossy().to_string()),
            format: config.format,
            audio_quality: config.audio_quality,
            ytdlp_channel: config.ytdlp_channel,
        }
    }

    fn to_config(&self) -> AppConfig {
        AppConfig {
            download_dir: self.download_dir.as_ref().map(PathBuf::from),
            format: self.format.clone(),
            audio_quality: self.audio_quality.clone(),
            language: "ko".to_string(),
            ytdlp_channel: self.ytdlp_channel.clone(),
        }
    }
}

fn string_to_format(format: &str) -> DownloadFormat {
    match format {
        "wav" => DownloadFormat::Wav,
        "m4a" => DownloadFormat::M4a,
        "flac" => DownloadFormat::Flac,
        "mp4" => DownloadFormat::Mp4,
        "webm" => DownloadFormat::Webm,
        _ => DownloadFormat::Mp3,
    }
}

fn download_event_from_status(
    status: DownloadStatus,
    current: usize,
    total: usize,
    title: &str,
) -> DownloadEvent {
    match status {
        DownloadStatus::Starting(message) => DownloadEvent {
            kind: "starting",
            current,
            total,
            percent: 0.0,
            title: title.to_string(),
            message,
        },
        DownloadStatus::Progress(percent, message) => DownloadEvent {
            kind: "progress",
            current,
            total,
            percent,
            title: title.to_string(),
            message,
        },
        DownloadStatus::Message(message) => DownloadEvent {
            kind: "message",
            current,
            total,
            percent: 0.0,
            title: title.to_string(),
            message,
        },
        DownloadStatus::Converting => DownloadEvent {
            kind: "converting",
            current,
            total,
            percent: 100.0,
            title: title.to_string(),
            message: "변환 중".to_string(),
        },
        DownloadStatus::Completed(message) => DownloadEvent {
            kind: "completed",
            current,
            total,
            percent: 100.0,
            title: message,
            message: "완료".to_string(),
        },
        DownloadStatus::Failed(message) => DownloadEvent {
            kind: "failed",
            current,
            total,
            percent: 0.0,
            title: title.to_string(),
            message,
        },
        DownloadStatus::Stopped => DownloadEvent {
            kind: "stopped",
            current,
            total,
            percent: 0.0,
            title: title.to_string(),
            message: "다운로드가 중지되었습니다.".to_string(),
        },
    }
}
