use std::fs;
use std::io::{Read, Write, copy};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub enum InitStatus {
    Starting(String),
    Downloading(f64, String),
    Extracting(String),
    Completed,
    Failed(String),
}

type ValidatedResult<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetOs {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetArch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DependencyTarget {
    os: TargetOs,
    arch: TargetArch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyPackage {
    Binary {
        url: &'static str,
    },
    Zip {
        url: &'static str,
        archive_name: &'static str,
    },
    TarXz {
        url: &'static str,
        archive_name: &'static str,
    },
}

impl DependencyTarget {
    fn current() -> ValidatedResult<Self> {
        Ok(Self {
            os: current_os()?,
            arch: current_arch()?,
        })
    }
}

pub fn init_dependencies(
    tx: std::sync::mpsc::Sender<InitStatus>,
    ytdlp_channel: crate::ytdlp::YtDlpChannel,
) {
    if let Err(error) = init_dependencies_inner(&tx, ytdlp_channel) {
        let _ = tx.send(InitStatus::Failed(error));
    }
}

fn init_dependencies_inner(
    tx: &std::sync::mpsc::Sender<InitStatus>,
    ytdlp_channel: crate::ytdlp::YtDlpChannel,
) -> ValidatedResult<()> {
    let app_dir = get_app_dir();
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("failed to create dependency folder: {}", e))?;

    let target = DependencyTarget::current()?;

    let ytdlp_path = get_ytdlp_path(&app_dir);
    if !is_runnable(&ytdlp_path, &["--version"]) {
        let _ = fs::remove_file(&ytdlp_path);
        download_ytdlp(&app_dir, target, tx)?;
    }

    let deno_path = get_deno_path(&app_dir);
    if !is_supported_deno(&deno_path) {
        let _ = fs::remove_file(&deno_path);
        download_deno(&app_dir, target, tx)?;
    }

    let ffmpeg_path = get_ffmpeg_path(&app_dir);
    if check_ffmpeg(&ffmpeg_path).is_err() {
        let _ = fs::remove_file(&ffmpeg_path);
        download_ffmpeg(&app_dir, target, tx)?;
    }

    let _ = tx.send(InitStatus::Starting("Checking yt-dlp updates".to_string()));
    match crate::ytdlp::update_ytdlp_channel(&ytdlp_path, ytdlp_channel) {
        Ok(message) => {
            let _ = tx.send(InitStatus::Starting(format!("yt-dlp: {}", message)));
        }
        Err(error) => {
            let _ = tx.send(InitStatus::Starting(format!(
                "yt-dlp update check failed: {}",
                error
            )));
        }
    }
    std::thread::sleep(Duration::from_millis(700));

    let _ = tx.send(InitStatus::Starting("Checking ffmpeg".to_string()));
    match check_ffmpeg(&ffmpeg_path) {
        Ok(message) => {
            let _ = tx.send(InitStatus::Starting(format!("ffmpeg: {}", message)));
        }
        Err(error) => return Err(format!("ffmpeg check failed after install: {}", error)),
    }
    std::thread::sleep(Duration::from_millis(700));

    let _ = tx.send(InitStatus::Completed);
    Ok(())
}

fn check_ffmpeg(ffmpeg_path: &Path) -> ValidatedResult<String> {
    let mut cmd = Command::new(ffmpeg_path);
    cmd.arg("-version");
    hide_console_window(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {}", e))?;

    if !output.status.success() {
        return Err("ffmpeg exited with an error".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version_line = stdout.lines().next().unwrap_or("ffmpeg detected");
    let display_msg = version_line.chars().take(42).collect::<String>();

    Ok(format!("OK ({})", display_msg))
}

fn is_runnable(path: &Path, args: &[&str]) -> bool {
    if !path.exists() {
        return false;
    }

    let mut cmd = Command::new(path);
    cmd.args(args);
    hide_console_window(&mut cmd);

    cmd.output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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

fn get_deno_path(app_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    return app_dir.join("deno.exe");
    #[cfg(not(target_os = "windows"))]
    return app_dir.join("deno");
}

fn is_supported_deno(deno_path: &Path) -> bool {
    if !deno_path.exists() {
        return false;
    }

    let mut cmd = Command::new(deno_path);
    cmd.arg("--version");
    hide_console_window(&mut cmd);

    let Ok(output) = cmd.output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(version) = stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
    else {
        return false;
    };

    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major >= 2)
}

fn download_file(
    url: &str,
    dest: &Path,
    tx: &std::sync::mpsc::Sender<InitStatus>,
    filename: &str,
) -> ValidatedResult<()> {
    use backoff::{ExponentialBackoff, retry};

    let _ = tx.send(InitStatus::Starting(format!("Preparing {}", filename)));

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {}", e))?;

    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(60)),
        initial_interval: Duration::from_secs(1),
        max_interval: Duration::from_secs(10),
        ..Default::default()
    };

    let response = retry(backoff, || {
        let _ = tx.send(InitStatus::Starting(format!("Downloading {}", filename)));

        client
            .get(url)
            .send()
            .map_err(backoff::Error::transient)
            .and_then(|response| {
                if response.status().is_success() {
                    Ok(response)
                } else {
                    Err(backoff::Error::permanent(
                        response.error_for_status().unwrap_err(),
                    ))
                }
            })
    })
    .map_err(|e| format!("download failed after retries: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut response = response;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = response.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| e.to_string())?;
        downloaded += bytes_read as u64;

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            let _ = tx.send(InitStatus::Downloading(percent, filename.to_string()));
        }
    }

    Ok(())
}

fn download_ytdlp(
    app_dir: &Path,
    target: DependencyTarget,
    tx: &std::sync::mpsc::Sender<InitStatus>,
) -> ValidatedResult<()> {
    let dest = get_ytdlp_path(app_dir);
    download_file(ytdlp_download_url(target)?, &dest, tx, "yt-dlp")?;
    make_executable(&dest)?;

    if !is_runnable(&dest, &["--version"]) {
        return Err("installed yt-dlp could not be executed".to_string());
    }

    Ok(())
}

fn download_deno(
    app_dir: &Path,
    target: DependencyTarget,
    tx: &std::sync::mpsc::Sender<InitStatus>,
) -> ValidatedResult<()> {
    let archive_path = app_dir.join("deno.zip");
    download_file(deno_download_url(target)?, &archive_path, tx, "Deno")?;

    let _ = tx.send(InitStatus::Extracting("Extracting Deno".to_string()));
    let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let dest_path = get_deno_path(app_dir);
    let mut extracted = false;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().replace('\\', "/");
        if !(name == "deno"
            || name == "deno.exe"
            || name.ends_with("/deno")
            || name.ends_with("/deno.exe"))
        {
            continue;
        }

        let mut outfile = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
        copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        extracted = true;
        break;
    }

    let _ = fs::remove_file(&archive_path);

    if !extracted {
        return Err("Deno executable was not found in the archive".to_string());
    }

    make_executable(&dest_path)?;

    if !is_supported_deno(&dest_path) {
        return Err("installed Deno does not satisfy yt-dlp's 2.x requirement".to_string());
    }

    Ok(())
}

fn download_ffmpeg(
    app_dir: &Path,
    target: DependencyTarget,
    tx: &std::sync::mpsc::Sender<InitStatus>,
) -> ValidatedResult<()> {
    match ffmpeg_package(target)? {
        DependencyPackage::Binary { url } => {
            let dest_path = get_ffmpeg_path(app_dir);
            download_file(url, &dest_path, tx, "ffmpeg")?;
            make_executable(&dest_path)?;
        }
        DependencyPackage::Zip { url, archive_name } => {
            let archive_path = app_dir.join(archive_name);
            download_file(url, &archive_path, tx, "ffmpeg archive")?;
            let _ = tx.send(InitStatus::Extracting("Extracting ffmpeg".to_string()));
            extract_ffmpeg_zip(&archive_path, app_dir)?;
            let _ = fs::remove_file(archive_path);
        }
        DependencyPackage::TarXz { url, archive_name } => {
            let archive_path = app_dir.join(archive_name);
            download_file(url, &archive_path, tx, "ffmpeg archive")?;
            let _ = tx.send(InitStatus::Extracting("Extracting ffmpeg".to_string()));
            extract_ffmpeg_tar_xz(&archive_path, app_dir)?;
            let _ = fs::remove_file(archive_path);
        }
    }

    check_ffmpeg(&get_ffmpeg_path(app_dir))?;
    Ok(())
}

fn current_os() -> ValidatedResult<TargetOs> {
    #[cfg(target_os = "windows")]
    {
        return Ok(TargetOs::Windows);
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(TargetOs::Macos);
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(TargetOs::Linux);
    }
    #[allow(unreachable_code)]
    Err("unsupported operating system".to_string())
}

fn current_arch() -> ValidatedResult<TargetArch> {
    #[cfg(target_arch = "x86_64")]
    {
        return Ok(TargetArch::X86_64);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return Ok(TargetArch::Aarch64);
    }
    #[allow(unreachable_code)]
    Err("unsupported CPU architecture".to_string())
}

fn ytdlp_download_url(target: DependencyTarget) -> ValidatedResult<&'static str> {
    match target.os {
        TargetOs::Windows => {
            Ok("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe")
        }
        TargetOs::Macos => {
            Ok("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos")
        }
        TargetOs::Linux => Ok("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"),
    }
}

fn deno_download_url(target: DependencyTarget) -> ValidatedResult<&'static str> {
    match (target.os, target.arch) {
        (TargetOs::Windows, TargetArch::X86_64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip",
        ),
        (TargetOs::Windows, TargetArch::Aarch64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-pc-windows-msvc.zip",
        ),
        (TargetOs::Macos, TargetArch::X86_64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-apple-darwin.zip",
        ),
        (TargetOs::Macos, TargetArch::Aarch64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-apple-darwin.zip",
        ),
        (TargetOs::Linux, TargetArch::X86_64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip",
        ),
        (TargetOs::Linux, TargetArch::Aarch64) => Ok(
            "https://github.com/denoland/deno/releases/latest/download/deno-aarch64-unknown-linux-gnu.zip",
        ),
    }
}

fn ffmpeg_package(target: DependencyTarget) -> ValidatedResult<DependencyPackage> {
    match (target.os, target.arch) {
        (TargetOs::Windows, TargetArch::X86_64) => Ok(DependencyPackage::Zip {
            url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
            archive_name: "ffmpeg.zip",
        }),
        (TargetOs::Windows, TargetArch::Aarch64) => {
            Err("automatic ffmpeg install is not available for Windows ARM64 yet".to_string())
        }
        (TargetOs::Macos, TargetArch::X86_64) => Ok(DependencyPackage::Binary {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/ffmpeg-darwin-x64",
        }),
        (TargetOs::Macos, TargetArch::Aarch64) => Ok(DependencyPackage::Binary {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/ffmpeg-darwin-arm64",
        }),
        (TargetOs::Linux, TargetArch::X86_64) => Ok(DependencyPackage::TarXz {
            url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
            archive_name: "ffmpeg.tar.xz",
        }),
        (TargetOs::Linux, TargetArch::Aarch64) => {
            Err("automatic ffmpeg install is not available for Linux ARM64 yet".to_string())
        }
    }
}

fn extract_ffmpeg_zip(archive_path: &Path, app_dir: &Path) -> ValidatedResult<()> {
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let dest_path = get_ffmpeg_path(app_dir);
    let mut extracted = false;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().replace('\\', "/");
        if !(name == "ffmpeg"
            || name == "ffmpeg.exe"
            || name.ends_with("/ffmpeg")
            || name.ends_with("/ffmpeg.exe"))
        {
            continue;
        }

        let mut outfile = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
        copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        extracted = true;
        break;
    }

    if !extracted {
        return Err("ffmpeg executable was not found in the archive".to_string());
    }

    make_executable(&dest_path)
}

fn extract_ffmpeg_tar_xz(archive_path: &Path, app_dir: &Path) -> ValidatedResult<()> {
    let status = Command::new("tar")
        .arg("-xf")
        .arg(archive_path)
        .arg("-C")
        .arg(app_dir)
        .status()
        .map_err(|e| format!("failed to run tar: {}", e))?;

    if !status.success() {
        return Err("failed to extract ffmpeg archive".to_string());
    }

    let dest_path = get_ffmpeg_path(app_dir);
    for entry in fs::read_dir(app_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir()
            || !entry.file_name().to_string_lossy().contains("ffmpeg")
        {
            continue;
        }

        let bin_path = entry.path().join("bin").join("ffmpeg");
        if bin_path.exists() {
            fs::rename(&bin_path, &dest_path).map_err(|e| e.to_string())?;
            make_executable(&dest_path)?;
            return Ok(());
        }
    }

    Err("ffmpeg executable was not found in the archive".to_string())
}

fn make_executable(path: &Path) -> ValidatedResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = path;
    }

    Ok(())
}

fn hide_console_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = cmd;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(os: TargetOs, arch: TargetArch) -> DependencyTarget {
        DependencyTarget { os, arch }
    }

    #[test]
    fn windows_installs_exe_ytdlp_and_zip_ffmpeg() {
        let target = target(TargetOs::Windows, TargetArch::X86_64);

        assert!(ytdlp_download_url(target).unwrap().ends_with("yt-dlp.exe"));
        assert_eq!(
            ffmpeg_package(target).unwrap(),
            DependencyPackage::Zip {
                url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
                archive_name: "ffmpeg.zip",
            }
        );
    }

    #[test]
    fn macos_uses_arch_specific_bundled_ffmpeg_binary() {
        assert_eq!(
            ffmpeg_package(target(TargetOs::Macos, TargetArch::X86_64)).unwrap(),
            DependencyPackage::Binary {
                url: "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/ffmpeg-darwin-x64",
            }
        );
        assert_eq!(
            ffmpeg_package(target(TargetOs::Macos, TargetArch::Aarch64)).unwrap(),
            DependencyPackage::Binary {
                url: "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/ffmpeg-darwin-arm64",
            }
        );
    }

    #[test]
    fn deno_downloads_match_platform_and_architecture() {
        assert!(
            deno_download_url(target(TargetOs::Macos, TargetArch::Aarch64))
                .unwrap()
                .contains("deno-aarch64-apple-darwin.zip")
        );
        assert!(
            deno_download_url(target(TargetOs::Windows, TargetArch::X86_64))
                .unwrap()
                .contains("deno-x86_64-pc-windows-msvc.zip")
        );
    }
}
