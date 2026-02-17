use std::io::{BufRead, BufReader};
use std::fs;
use image::{GenericImageView, ImageFormat};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
                "--write-thumbnail".to_string(),
                "--convert-thumbnails".to_string(), "jpg".to_string(),
            ]);
        }
        DownloadFormat::Wav => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "wav".to_string(),
                "--write-thumbnail".to_string(),
                "--convert-thumbnails".to_string(), "jpg".to_string(),
            ]);
        }
        DownloadFormat::M4a => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "m4a".to_string(),
                "--write-thumbnail".to_string(),
                "--convert-thumbnails".to_string(), "jpg".to_string(),
            ]);
        }
        DownloadFormat::Flac => {
            args.extend_from_slice(&[
                "-x".to_string(),
                "--audio-format".to_string(), "flac".to_string(),
                "--write-thumbnail".to_string(),
                "--convert-thumbnails".to_string(), "jpg".to_string(),
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
    
    // 중지 요청 추적을 위한 atomic flag
    let stopped = Arc::new(AtomicBool::new(false));
    let stopped_for_killer = stopped.clone();
    
    // 1. Killer 스레드: 중지 신호 감시
    let child_for_killer = child_shared.clone();
    thread::spawn(move || {
        if stop_signal.recv().is_ok() {
            // 중지 플래그 설정
            stopped_for_killer.store(true, Ordering::SeqCst);
            // 신호 수신 시 프로세스 kill
            if let Ok(mut c) = child_for_killer.lock() {
                 let _ = c.kill();
            }
        }
    });

    // 2. 메인 로직: stdout 읽기
    // Mutex를 잠깐 잠그고 stdout/stderr을 가져옴 (option take)
    let (stdout, stderr) = {
        let mut c = child_shared.lock().unwrap();
        (c.stdout.take(), c.stderr.take())
    };

    let stderr_tail = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_tail_for_thread = stderr_tail.clone();
    let stderr_reader_handle = thread::spawn(move || {
        if let Some(err) = stderr {
            let reader = BufReader::new(err);
            for line in reader.lines().map_while(Result::ok) {
                let mut tail = stderr_tail_for_thread.lock().unwrap();
                tail.push(line);
                if tail.len() > 5 {
                    tail.remove(0);
                }
            }
        }
    });

    let mut detected_audio_output: Option<PathBuf> = None;

    if let Some(out) = stdout {
        let reader = BufReader::new(out);
        for line in reader.lines() {
            if let Ok(line) = line {
                if detected_audio_output.is_none() {
                    detected_audio_output = parse_extract_audio_output_line(&line);
                }

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
    let _ = stderr_reader_handle.join();

    // 중지 신호가 왔는지 확인
    let was_stopped = stopped.load(Ordering::SeqCst);

    match status_result {
        Ok(status) => {
            if status.success() {
                if is_audio_format(&config.format) {
                    let output_file = resolve_audio_output_path(
                        &config.output_dir,
                        &sanitized_title,
                        &config.format,
                        detected_audio_output,
                    );

                    match resolve_thumbnail_path(&output_file, &config.output_dir, &sanitized_title) {
                        Some(thumb_file) => {
                            let _ = crop_thumbnail_image_to_square(&thumb_file);
                            match embed_thumbnail_to_audio(&output_file, &thumb_file, &new_path) {
                                Ok(()) => {
                                    let _ = fs::remove_file(thumb_file);
                                }
                                Err(err) => {
                                    let _ = tx.send(DownloadStatus::Failed(format!(
                                        "앨범아트 임베드 실패: {}",
                                        err
                                    )));
                                    return;
                                }
                            }
                        }
                        None => {
                            let _ = tx.send(DownloadStatus::Failed(
                                "앨범아트 파일을 찾지 못했습니다".to_string(),
                            ));
                            return;
                        }
                    }
                }
                let _ = tx.send(DownloadStatus::Completed(title));
            } else if was_stopped {
                // 사용자가 중지를 요청한 경우
                let _ = tx.send(DownloadStatus::Stopped);
            } else {
                // 실제 오류
                let tail = stderr_tail.lock().unwrap();
                let detail = if tail.is_empty() {
                    "다운로드 실패".to_string()
                } else {
                    format!("다운로드 실패: {}", tail.join(" | "))
                };
                let _ = tx.send(DownloadStatus::Failed(detail));
            }
        }
        Err(_) => {
            if was_stopped {
                let _ = tx.send(DownloadStatus::Stopped);
            } else {
                let _ = tx.send(DownloadStatus::Failed("프로세스 대기 오류".to_string()));
            }
        }
    }
}

fn is_audio_format(format: &DownloadFormat) -> bool {
    matches!(
        format,
        DownloadFormat::Mp3 | DownloadFormat::Wav | DownloadFormat::M4a | DownloadFormat::Flac
    )
}

fn audio_output_path(output_dir: &PathBuf, sanitized_title: &str, format: &DownloadFormat) -> PathBuf {
    let ext = match format {
        DownloadFormat::Mp3 => "mp3",
        DownloadFormat::Wav => "wav",
        DownloadFormat::M4a => "m4a",
        DownloadFormat::Flac => "flac",
        _ => "",
    };

    output_dir.join(format!("{}.{}", sanitized_title, ext))
}

fn resolve_audio_output_path(
    output_dir: &PathBuf,
    sanitized_title: &str,
    format: &DownloadFormat,
    detected_path: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = detected_path {
        if path.exists() {
            return path;
        }
    }

    let expected = audio_output_path(output_dir, sanitized_title, format);
    if expected.exists() {
        return expected;
    }

    let ext = match format {
        DownloadFormat::Mp3 => "mp3",
        DownloadFormat::Wav => "wav",
        DownloadFormat::M4a => "m4a",
        DownloadFormat::Flac => "flac",
        _ => "",
    };

    let mut latest_match: Option<(std::time::SystemTime, PathBuf)> = None;

    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_ext = path.extension().and_then(|value| value.to_str());
            if file_ext != Some(ext) {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default();

            if !stem.starts_with(sanitized_title) {
                continue;
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            match &latest_match {
                Some((latest_time, _)) if &modified <= latest_time => {}
                _ => latest_match = Some((modified, path)),
            }
        }
    }

    latest_match.map(|(_, path)| path).unwrap_or(expected)
}

fn parse_extract_audio_output_line(line: &str) -> Option<PathBuf> {
    if let Some(path) = line.strip_prefix("[ExtractAudio] Destination: ") {
        return Some(PathBuf::from(path.trim()));
    }

    let prefix = "[ExtractAudio] Post-process file ";
    let suffix = " exists, skipping";
    if line.starts_with(prefix) && line.ends_with(suffix) {
        let start = prefix.len();
        let end = line.len() - suffix.len();
        return Some(PathBuf::from(line[start..end].trim()));
    }

    None
}

fn resolve_thumbnail_path(
    audio_path: &PathBuf,
    output_dir: &PathBuf,
    sanitized_title: &str,
) -> Option<PathBuf> {
    let same_stem = audio_path.with_extension("jpg");
    if same_stem.exists() {
        return Some(same_stem);
    }

    let mut latest_match: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jpg") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if !stem.starts_with(sanitized_title) {
                continue;
            }

            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            match &latest_match {
                Some((latest_time, _)) if &modified <= latest_time => {}
                _ => latest_match = Some((modified, path)),
            }
        }
    }

    latest_match.map(|(_, path)| path)
}

fn crop_thumbnail_image_to_square(image_path: &PathBuf) -> std::io::Result<()> {
    let image = image::open(image_path).map_err(std::io::Error::other)?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Ok(());
    }

    let side = width.min(height);
    let left = (width - side) / 2;
    let top = (height - side) / 2;
    let cropped = image.crop_imm(left, top, side, side);
    cropped
        .save_with_format(image_path, ImageFormat::Jpeg)
        .map_err(std::io::Error::other)
}

fn embed_thumbnail_to_audio(
    audio_path: &PathBuf,
    thumbnail_path: &PathBuf,
    path_env: &str,
) -> Result<(), String> {
    if !audio_path.exists() {
        return Err("오디오 파일을 찾지 못했습니다".to_string());
    }
    if !thumbnail_path.exists() {
        return Err("썸네일 파일을 찾지 못했습니다".to_string());
    }

    let ext = audio_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let stem = audio_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let tmp_path = audio_path.with_file_name(format!("{}.tmp.{}", stem, ext));

    let output = Command::new("ffmpeg")
        .env("PATH", path_env)
        .arg("-y")
        .arg("-i")
        .arg(audio_path)
        .arg("-i")
        .arg(thumbnail_path)
        .arg("-map")
        .arg("0:a")
        .arg("-map")
        .arg("1:v:0")
        .arg("-map_metadata")
        .arg("0")
        .arg("-c:a")
        .arg("copy")
        .arg("-c:v")
        .arg("mjpeg")
        .arg("-metadata:s:v")
        .arg("title=Album cover")
        .arg("-metadata:s:v")
        .arg("comment=Cover (front)")
        .arg("-disposition:v:0")
        .arg("attached_pic")
        .arg(&tmp_path)
        .output()
        .map_err(|e| format!("ffmpeg 실행 실패: {}", e))?;

    if !output.status.success() || !tmp_path.exists() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        let _ = fs::remove_file(&tmp_path);
        return Err(if err.trim().is_empty() {
            "ffmpeg가 임베드에 실패했습니다".to_string()
        } else {
            err
        });
    }

    fs::remove_file(audio_path).map_err(|e| format!("원본 파일 교체 실패: {}", e))?;
    fs::rename(tmp_path, audio_path).map_err(|e| format!("임시 파일 교체 실패: {}", e))?;
    Ok(())
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
