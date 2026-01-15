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

pub fn init_dependencies(tx: std::sync::mpsc::Sender<InitStatus>) {
    let app_dir = get_app_dir();
    if !app_dir.exists() {
        if let Err(e) = fs::create_dir_all(&app_dir) {
            let _ = tx.send(InitStatus::Failed(format!("폴더 생성 실패: {}", e)));
            return;
        }
    }

    // 1. Check yt-dlp
    let ytdlp_path = get_ytdlp_path(&app_dir);
    if !ytdlp_path.exists() {
        if let Err(e) = download_ytdlp(&app_dir, &tx) {
            let _ = tx.send(InitStatus::Failed(format!("yt-dlp 다운로드 실패: {}", e)));
            return;
        }
    }

    // 2. Check ffmpeg
    let ffmpeg_path = get_ffmpeg_path(&app_dir);
    if !ffmpeg_path.exists() {
        if let Err(e) = download_ffmpeg(&app_dir, &tx) {
            let _ = tx.send(InitStatus::Failed(format!("ffmpeg 다운로드 실패: {}", e)));
            return;
        }
    }

    let _ = tx.send(InitStatus::Completed);
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
    let _ = tx.send(InitStatus::Starting(format!("{} 다운로드 준비...", filename)));
    
    let client = reqwest::blocking::Client::new();
    let mut response = client.get(url).send().map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("서버 응답 오류: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    use std::io::Read;
    use std::io::Write;

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
    let _ = tx.send(InitStatus::Starting("ffmpeg 확인 중...".to_string()));

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

    let _ = tx.send(InitStatus::Extracting("ffmpeg 압축 해제 중...".to_string()));

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

type ValidatedResult<T> = Result<T, String>;
