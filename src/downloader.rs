use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadFormat {
    Mp3,
    Wav,
    M4a,
    Flac,
    Mp4,
    Webm,
}

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub url: String,
    pub format: DownloadFormat,
    pub audio_quality: String,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Starting(String),     // message
    Progress(f64, String), // percent, speed/status
    Converting,
    Completed(String),    // filename
    Failed(String),       // error message
    Stopped,              // [NEW] 중단됨
}

pub fn download_video(
    config: DownloadConfig, 
    title: String, 
    tx: Sender<DownloadStatus>,
    stop_signal: Receiver<()> // [NEW] 중지 신호
) {
    let ytdlp = crate::playlist::get_ytdlp_path();
    
    // 파일명 살균 및 템플릿 설정
    let sanitized_title = sanitize_filename(&title);
    
    // ffmpeg 경로 설정을 위한 PATH 업데이트
    #[cfg(target_os = "windows")]
    let new_path = {
        let app_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rust-yt");
        let current_path = std::env::var("PATH").unwrap_or_default();
        format!("{};{}", app_dir.display(), current_path)
    };
    #[cfg(target_os = "macos")]
    let new_path = {
        let current_path = std::env::var("PATH").unwrap_or_default();
        // GUI 앱은 PATH가 제대로 설정되지 않을 수 있으므로 homebrew 경로 추가
        format!("{}:/opt/homebrew/bin:/usr/local/bin:{}", current_path, std::env::var("HOME").unwrap_or_default() + "/.cargo/bin")
    };
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let new_path = std::env::var("PATH").unwrap_or_default();

    let output_template = match config.format {
        DownloadFormat::Mp3 | DownloadFormat::Wav | DownloadFormat::M4a | DownloadFormat::Flac => {
            config.output_dir.join(format!("{}.%(ext)s", sanitized_title))
        }
        _ => config.output_dir.join(format!("{}.%(ext)s", sanitized_title)), // Video formats mainly
    };

    let output_str = output_template.to_string_lossy().to_string();

    let mut args = vec![
        "--no-playlist".to_string(),
        "--newline".to_string(),
        "--progress".to_string(),
        "--embed-thumbnail".to_string(), // [NEW] 썸네일 포함
        "--add-metadata".to_string(),    // [NEW] 메타데이터 포함
        "-o".to_string(),
        output_str,
    ];

    match config.format {
        DownloadFormat::Mp3 => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "mp3".to_string(),
                "--audio-quality".to_string(), config.audio_quality,
            ]);
        }
        DownloadFormat::Wav => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "wav".to_string(),
            ]);
        }
        DownloadFormat::M4a => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "m4a".to_string(),
            ]);
        }
        DownloadFormat::Flac => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "flac".to_string(),
            ]);
        }
        DownloadFormat::Mp4 => {
            args.extend_from_slice(&[
                "-f".to_string(), "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string(),
                "--merge-output-format".to_string(), "mp4".to_string(),
            ]);
        }
        DownloadFormat::Webm => {
            args.extend_from_slice(&[
                "-f".to_string(), "bestvideo[ext=webm]+bestaudio/best".to_string(),
                "--merge-output-format".to_string(), "webm".to_string(),
            ]);
        }
    }

    // URL은 마지막에 추가
    args.push(config.url);

    let _ = tx.send(DownloadStatus::Starting("다운로드 시작...".to_string()));

    let mut command = Command::new(&ytdlp);
    command.env("PATH", &new_path)
           .args(&args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(DownloadStatus::Failed(format!("실행 실패: {}", e)));
                return;
            }
        };

    // Child를 Arc<Mutex>로 감싸서 공유
    let child_shared = Arc::new(Mutex::new(child));
    
    // 1. Killer 스레드: 중지 신호 감시
    let child_for_killer = child_shared.clone();
    thread::spawn(move || {
        if stop_signal.recv().is_ok() {
            // 신호 수신 시 프로세스 kill
            if let Ok(mut c) = child_for_killer.lock() {
                 let _ = c.kill();
            }
        }
    });

    // 2. 메인 로직: stdout 읽기
    // Mutex를 잠깐 잠그고 stdout을 가져옴 (option take)
    let stdout = {
        let mut c = child_shared.lock().unwrap();
        c.stdout.take()
    };

    if let Some(out) = stdout {
        let reader = BufReader::new(out);
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("[download]") && line.contains("%") {
                    if let Some(percent_str) = line.split_whitespace().find(|s| s.ends_with('%')) {
                        if let Ok(percent) = percent_str.trim_end_matches('%').parse::<f64>() {
                            let speed = line.split_whitespace()
                                .find(|s| s.ends_with("/s"))
                                .unwrap_or("")
                                .to_string();
                            let _ = tx.send(DownloadStatus::Progress(percent, speed));
                        }
                    }
                }
                
                if line.contains("[ExtractAudio]") || line.contains("[Merger]") {
                    let _ = tx.send(DownloadStatus::Converting);
                }
            }
        }
    }

    // 프로세스 종료 대기
    // 이미 kill 되었을 수도 있음
    let status_result = {
        let mut c = child_shared.lock().unwrap();
        c.wait()
    };

    match status_result {
        Ok(status) => {
            if status.success() {
                let _ = tx.send(DownloadStatus::Completed(title));
            } else {
                // kill 된 경우도 포함될 수 있음 (Windows에서는 kill 시 종료 코드 다름)
                // 명확히 구분하기 어렵지만, 사용자가 중단을 눌렀다면 UI측에서 Stopped 처리
                let _ = tx.send(DownloadStatus::Failed("다운로드 실패 (또는 중단)".to_string()));
            }
        }
        Err(_) => {
             let _ = tx.send(DownloadStatus::Failed("프로세스 대기 오류".to_string()));
        }
    }
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-_()[].,!&'".contains(*c))
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
