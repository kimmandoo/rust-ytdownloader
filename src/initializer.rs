use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub enum InitStatus {
    Starting(String),
    Downloading(f64, String), // percent, filename
    Extracting(String),
    Completed,
    Failed(String),
}

// Assuming ValidatedResult is defined elsewhere, e.g., type ValidatedResult<T> = Result<T, String>;
type ValidatedResult<T> = Result<T, String>;

pub fn init_dependencies(tx: std::sync::mpsc::Sender<InitStatus>) {
    let app_dir = get_app_dir();
    if !app_dir.exists() {
        if let Err(e) = fs::create_dir_all(&app_dir) {
            let _ = tx.send(InitStatus::Failed(rust_i18n::t!("initialization.folder_error", error = e.to_string()).to_string()));
            return;
        }
    }

    // 1. Check yt-dlp
    let ytdlp_path = get_ytdlp_path(&app_dir);
    if !ytdlp_path.exists() {
        if let Err(e) = download_ytdlp(&app_dir, &tx) {
            let _ = tx.send(InitStatus::Failed(rust_i18n::t!("initialization.ytdlp_download_fail", error = e).to_string()));
            return;
        }
    }

    // 2. Check ffmpeg
    let ffmpeg_path = get_ffmpeg_path(&app_dir);
    if !ffmpeg_path.exists() {
        if let Err(e) = download_ffmpeg(&app_dir, &tx) {
            let _ = tx.send(InitStatus::Failed(rust_i18n::t!("initialization.ffmpeg_download_fail", error = e).to_string()));
            return;
        }
    }

    // 3. Update Check (Non-fatal)
    // yt-dlp 업데이트 확인
    let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.ytdlp_update_check").to_string()));
    match update_ytdlp(&ytdlp_path) {
        Ok(msg) => {
            let _ = tx.send(InitStatus::Starting(format!("yt-dlp: {}", msg)));
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
        Err(e) => {
            let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.ytdlp_update_fail", error = e).to_string()));
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }

    // ffmpeg 작동 확인
    let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.ffmpeg_check").to_string()));
    match check_ffmpeg(&ffmpeg_path) {
        Ok(msg) => {
            let _ = tx.send(InitStatus::Starting(format!("ffmpeg: {}", msg)));
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
        Err(e) => {
            let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.ffmpeg_check_fail", error = e).to_string()));
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }

    let _ = tx.send(InitStatus::Completed);
}

fn update_ytdlp(ytdlp_path: &Path) -> ValidatedResult<String> {
    let mut cmd = Command::new(ytdlp_path);
    cmd.arg("-U");
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd.output()
        .map_err(|e| format!("실행 실패: {}", e))?;

    if !output.status.success() {
         let stderr = String::from_utf8_lossy(&output.stderr);
         return Err(format!("업데이트 실패: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Find the line containing "up to date" or "Updated"
    let status_line = stdout.lines()
        .find(|l| l.contains("up to date") || l.contains("Updated"))
        .unwrap_or("업데이트 확인 완료");
        
    Ok(status_line.trim().to_string())
}

fn check_ffmpeg(ffmpeg_path: &Path) -> ValidatedResult<String> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.arg("-version");
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd.output()
        .map_err(|e| format!("실행 실패: {}", e))?;

    if !output.status.success() {
         return Err("ffmpeg 실행 중 오류 발생".to_string());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_line = stdout.lines().next().unwrap_or("ffmpeg 감지됨");
    
    // 버전 정보만 간략히 추출 (예: ffmpeg version n6.0 ... -> version n6.0)
    let display_msg = if version_line.len() > 30 {
        &version_line[..30]
    } else {
        version_line
    };
    
    Ok(format!("정상 작동 ({})", display_msg))
}

fn get_app_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-yt")
}

fn get_ytdlp_path(app_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    return app_dir.join("yt-dlp.exe");
    #[cfg(not(target_os = "windows"))]
    return app_dir.join("yt-dlp");
}

fn get_ffmpeg_path(app_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    return app_dir.join("ffmpeg.exe");
    #[cfg(not(target_os = "windows"))]
    return app_dir.join("ffmpeg");
}

fn download_file(url: &str, dest: &Path, tx: &std::sync::mpsc::Sender<InitStatus>, filename: &str) -> ValidatedResult<()> {
    use backoff::{ExponentialBackoff, retry};
    use std::time::Duration;

    let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.downloading_prep", file = filename).to_string()));
    
    // 타임아웃 설정된 클라이언트 생성
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(300)) // 5분
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    // 재시도 설정 (최대 3회, 지수 백오프)
    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(60)),
        initial_interval: Duration::from_secs(1),
        max_interval: Duration::from_secs(10),
        ..Default::default()
    };

    let url_owned = url.to_string();
    let filename_owned = filename.to_string();
    let tx_clone = tx.clone();

    // 재시도 로직으로 HTTP 요청
    let response = retry(backoff, || {
        let _ = tx_clone.send(InitStatus::Starting(rust_i18n::t!("initialization.downloading_attempt", file = filename_owned).to_string()));
        
        client.get(&url_owned)
            .send()
            .map_err(|e| {
                let _ = tx_clone.send(InitStatus::Starting(rust_i18n::t!("initialization.downloading_retry", file = filename_owned).to_string()));
                backoff::Error::transient(e)
            })
            .and_then(|resp| {
                if resp.status().is_success() {
                    Ok(resp)
                } else {
                    Err(backoff::Error::permanent(
                        reqwest::Error::from(resp.error_for_status().unwrap_err())
                    ))
                }
            })
    }).map_err(|e| rust_i18n::t!("initialization.download_failed_retry", error = e).to_string())?;

    let total_size = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    use std::io::Read;
    use std::io::Write;

    let mut response = response;
    loop {
        let bytes_read = response.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read]).map_err(|e| e.to_string())?;
        downloaded += bytes_read as u64;

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            let _ = tx.send(InitStatus::Downloading(percent, filename.to_string()));
        }
    }

    Ok(())
}

fn download_ytdlp(app_dir: &Path, tx: &std::sync::mpsc::Sender<InitStatus>) -> ValidatedResult<()> {
    #[cfg(target_os = "linux")]
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp";
    #[cfg(target_os = "macos")]
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos";
    #[cfg(target_os = "windows")]
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

    let dest = get_ytdlp_path(app_dir);
    download_file(url, &dest, tx, "yt-dlp")?;

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).unwrap();
    }

    Ok(())
}

fn download_ffmpeg(app_dir: &Path, tx: &std::sync::mpsc::Sender<InitStatus>) -> ValidatedResult<()> {
    let _ = tx.send(InitStatus::Starting(rust_i18n::t!("initialization.ffmpeg_check").to_string()));

    #[cfg(target_os = "linux")]
    let (url, archive_name) = (
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
        "ffmpeg.tar.xz"
    );
     #[cfg(target_os = "macos")]
    let (url, archive_name) = (
        "https://evermeet.cx/ffmpeg/ffmpeg-6.0.zip", // Note: macOS builds vary, using a common one
        "ffmpeg.zip"
    );
    #[cfg(target_os = "windows")]
     let (url, archive_name) = (
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
        "ffmpeg.zip"
    );

    let archive_path = app_dir.join(archive_name);
    
    // macOS needs special handling or a better URL, using a simpler zip for now if possible or just skipping for brevity on this complex platform
    // Simplified for Linux/Windows primarily as requested
    
    // For macOS, simplistic implementation might fail due to lack of static builds or gatekeeper. 
    // Let's assume user has it or we use a static build.
    // Using BtbN for Linux/Windows is reliable. 
    
    download_file(url, &archive_path, tx, "ffmpeg archive")?;

    let _ = tx.send(InitStatus::Extracting(rust_i18n::t!("initialization.extracting", file = "ffmpeg").to_string()));

    if archive_name.ends_with(".zip") {
        let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();

            if name.ends_with("ffmpeg") || name.ends_with("ffmpeg.exe") {
                 let dest_path = get_ffmpeg_path(app_dir);
                 let mut outfile = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
                 copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
                 
                 #[cfg(not(target_os = "windows"))]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&dest_path).unwrap().permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&dest_path, perms).unwrap();
                }
            }
        }
    } else if archive_name.ends_with(".tar.xz") {
         // tar.xz extraction requires xz2 crate or command line
         // Simpler to just use Command for tar if available (Linux usually has tar)
         let status = Command::new("tar")
            .arg("-xf")
            .arg(&archive_path)
            .arg("-C")
            .arg(app_dir)
            .status()
            .map_err(|e| format!("tar 실행 실패: {}", e))?;
            
        if !status.success() {
             return Err("tar 압축 해제 실패".to_string());
        }
        
        // Find ffmpeg binary in the extracted folder and move it
        // The structure is usually ffmpeg-master-latest-linux64-gpl/bin/ffmpeg
        for entry in fs::read_dir(app_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() && entry.file_name().to_string_lossy().contains("ffmpeg") {
                 let bin_path = entry.path().join("bin").join("ffmpeg");
                 if bin_path.exists() {
                     fs::rename(bin_path, get_ffmpeg_path(app_dir)).unwrap();
                 }
            }
        }
    }

    // Cleanup
    let _ = fs::remove_file(archive_path);
    
    Ok(())
}


