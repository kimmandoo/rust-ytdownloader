#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rust_yt::playlist::{fetch_playlist_info, PlaylistInfo, VideoEntry};
use rust_yt::downloader::{download_video, DownloadConfig, DownloadFormat, DownloadStatus};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // 폰트 설정 (임베디드 폰트)
    // 윈도우/리눅스 모두에서 한글 깨짐을 방지하기 위해 폰트를 바이너리에 포함
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_resizable(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx); // [NEW] 이미지 로더 설치
            Ok(Box::new(MyApp::default()))
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // NanumGothic.ttf를 바이너리에 포함 (컴파일 시점에 assets/fonts/NanumGothic.ttf가 있어야 함)
    // src/main.rs 기준이므로 ../assets 가 맞음
    fonts.font_data.insert(
        "NanumGothic".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NanumGothic.ttf"
        ))),
    );

    // Proportional 폰트의 최우선 순위로 설정
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NanumGothic".to_owned());

    // Monospace 폰트의 최우선 순위로 설정 (선택사항)
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "NanumGothic".to_owned());

    ctx.set_fonts(fonts);
}

#[derive(Debug)]
enum AppState {
    SetPath, // [NEW] 초기 경로 설정
    Input,
    Analyzing,
    Ready,
    Downloading,
    Finished,
}

struct MyApp {
    download_dir: PathBuf, // [NEW] 저장 경로
    url: String,
    format: DownloadFormat,
    state: AppState,
    playlist_info: Option<PlaylistInfo>,
    error_msg: Option<String>,
    
    // 다운로드 관련
    download_queue: Vec<VideoEntry>,
    current_download_idx: usize,
    progress: f64,
    progress_text: String,
    
    // 비동기 통신
    tx_ui: Sender<UiMessage>,
    rx_ui: Receiver<UiMessage>,
    stop_tx: Option<Sender<()>>, // [NEW] 중지 신호 송신
}

enum UiMessage {
    AnalysisDone(Result<PlaylistInfo, String>),
    DownloadProgress(DownloadStatus),
}

impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            download_dir: PathBuf::new(), // 초기화
            url: String::new(),
            format: DownloadFormat::Mp3,
            state: AppState::SetPath, // [NEW] 시작 상태 변경
            playlist_info: None,
            error_msg: None,
            download_queue: Vec::new(),
            current_download_idx: 0,
            progress: 0.0,
            progress_text: String::new(),
            tx_ui: tx,
            rx_ui: rx,
            stop_tx: None,
        }
    }
}

impl MyApp {
    fn start_analysis(&mut self) {
        let url = self.url.clone();
        let tx = self.tx_ui.clone();
        
        self.state = AppState::Analyzing;
        self.error_msg = None;
        
        thread::spawn(move || {
            let result = fetch_playlist_info(&url);
            let _ = tx.send(UiMessage::AnalysisDone(result));
        });
    }

    fn start_download(&mut self) -> Result<(), String> {
        let info = self.playlist_info.as_ref().ok_or("정보 없음")?;
        
        // 선택된 영상만 필터링
        self.download_queue = info.entries.iter()
            .filter(|e| e.selected)
            .cloned()
            .collect();
            
        if self.download_queue.is_empty() {
            return Err("선택된 영상이 없습니다.".to_string());
        }

        self.current_download_idx = 0;
        self.state = AppState::Downloading;
        self.download_next();
        Ok(())
    }
    
    fn stop_download(&mut self) {
        if let Some(tx) = &self.stop_tx {
            let _ = tx.send(());
        }
        // stop_tx는 즉시 해제하지 않고, 스레드가 종료되어 Failed/Stopped 메시지를 보낼 때까지 기다리거나
        // UI 반응성을 위해 즉시 상태 변경
        self.progress_text = "중지 중...".to_string();
    }

    fn download_next(&mut self) {
        if self.current_download_idx >= self.download_queue.len() {
            self.state = AppState::Finished;
            self.progress_text = "모든 다운로드 완료!".to_string();
            self.progress = 1.0;
            self.stop_tx = None;
            return;
        }

        let video = self.download_queue[self.current_download_idx].clone();
        let tx = self.tx_ui.clone();
        
        let config = DownloadConfig {
            url: video.url.clone(),
            format: self.format.clone(),
            audio_quality: "320K".to_string(),
            output_dir: self.download_dir.clone(), // [NEW] 선택된 경로 사용
        };

        // UI 초기화
        self.progress = 0.0;
        self.progress_text = format!("준비 중: {}", video.title);
        
        // 중지 채널 생성
        let (stop_tx, stop_rx) = channel();
        self.stop_tx = Some(stop_tx);
        
        thread::spawn(move || {
            let (tx_internal, rx_internal) = channel();
            
            // 별도 스레드에서 다운로드 실행 (tx_internal 소유권 이동)
            let config_clone = config.clone();
            let title_clone = video.title.clone();
            let tx_internal_clone = tx_internal.clone();
            
            thread::spawn(move || {
                download_video(config_clone, title_clone, tx_internal_clone, stop_rx);
            });

            // 중계 루프
            while let Ok(status) = rx_internal.recv() {
                 match tx.send(UiMessage::DownloadProgress(status)) {
                     Ok(_) => {},
                     Err(_) => break, // UI가 닫히면 종료
                 }
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 메시지 처리
        while let Ok(msg) = self.rx_ui.try_recv() {
            match msg {
                UiMessage::AnalysisDone(result) => {
                    match result {
                        Ok(info) => {
                            self.playlist_info = Some(info);
                            self.state = AppState::Ready;
                        }
                        Err(e) => {
                            self.error_msg = Some(e);
                            self.state = AppState::Input;
                        }
                    }
                }
                UiMessage::DownloadProgress(status) => {
                    match status {
                        DownloadStatus::Starting(msg) => {
                            self.progress_text = msg;
                            self.progress = 0.0;
                        }
                        DownloadStatus::Progress(p, speed) => {
                            self.progress = p / 100.0;
                            self.progress_text = format!("{:.1}% ({})", p, speed);
                        }
                        DownloadStatus::Converting => {
                            self.progress_text = "변환 중...".to_string();
                        }
                        DownloadStatus::Completed(_) => {
                            self.current_download_idx += 1;
                            self.download_next();
                        }
                        DownloadStatus::Failed(e) => {
                            if self.progress_text == "중지 중..." {
                                self.state = AppState::Ready;
                                self.progress_text = "다운로드가 중지되었습니다.".to_string();
                            } else {
                                self.progress_text = format!("오류: {}", e);
                                self.error_msg = Some(format!("다운로드 중단: {}", e));
                                self.state = AppState::Ready;
                            }
                            self.stop_tx = None;
                        }
                        DownloadStatus::Stopped => {
                            self.state = AppState::Ready;
                            self.progress_text = "다운로드가 중지되었습니다.".to_string();
                            self.stop_tx = None;
                        }
                    }
                }
            }
        }

        // 0. 초기 경로 설정 화면
        if matches!(self.state, AppState::SetPath) {
             egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("🎬 YouTube Downloader");
                    ui.add_space(50.0);
                    ui.label("다운로드할 폴더를 선택해주세요.");
                    ui.add_space(20.0);
                    if ui.button("폴더 선택하기").clicked() {
                         if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.download_dir = path;
                            self.state = AppState::Input;
                        }
                    }
                });
            });
            return;
        }

        // 1. Top Panel (설정 및 입력)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.heading("🎬 YouTube Downloader");
            ui.add_space(5.0);

            // 경로 등
            ui.horizontal(|ui| {
                ui.label(format!("저장 위치: {}", self.download_dir.display()));
                if ui.button("변경").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.download_dir = path;
                    }
                }
            });
            ui.separator();

            // URL 입력
            ui.horizontal(|ui| {
                ui.label("URL:");
                let text_edit = ui.text_edit_singleline(&mut self.url);
                if self.state.is_input() || matches!(self.state, AppState::Ready | AppState::Finished) {
                    if ui.button("분석").clicked() || (text_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))) {
                        if !self.url.trim().is_empty() {
                            self.start_analysis();
                        }
                    }
                }
            });

            ui.add_space(5.0);

            // 형식 선택
            ui.horizontal(|ui| {
                ui.label("형식:");
                egui::ComboBox::from_id_salt("format_combo")
                    .selected_text(match self.format {
                        DownloadFormat::Mp3 => "🎵 Audio (MP3)",
                        DownloadFormat::Wav => "🎵 Audio (WAV)",
                        DownloadFormat::M4a => "🎵 Audio (M4A)",
                        DownloadFormat::Flac => "🎵 Audio (FLAC)",
                        DownloadFormat::Mp4 => "🎬 Video (MP4)",
                        DownloadFormat::Webm => "🎬 Video (WEBM)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.format, DownloadFormat::Mp3, "🎵 Audio (MP3)");
                        ui.selectable_value(&mut self.format, DownloadFormat::Wav, "🎵 Audio (WAV)");
                        ui.selectable_value(&mut self.format, DownloadFormat::M4a, "🎵 Audio (M4A)");
                        ui.selectable_value(&mut self.format, DownloadFormat::Flac, "🎵 Audio (FLAC)");
                        ui.separator();
                        ui.selectable_value(&mut self.format, DownloadFormat::Mp4, "🎬 Video (MP4)");
                        ui.selectable_value(&mut self.format, DownloadFormat::Webm, "🎬 Video (WEBM)");
                    });
            });

             // 로딩 상태 (Top Panel에 표시)
            if matches!(self.state, AppState::Analyzing) {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("영상 정보를 분석 중입니다...");
                });
            }
            
             ui.add_space(5.0);
        });

        // 2. Bottom Panel (액션, 상태, 프로그레스)
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            
            // 에러 메시지
            if let Some(err) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, format!("⚠️ {}", err));
                ui.separator();
            }

            // 다운로드 컨트롤
            match self.state {
                AppState::Ready => {
                    let btn_text = if let Some(info) = &self.playlist_info {
                        let count = info.entries.iter().filter(|e| e.selected).count();
                        if count > 0 {
                            format!("{}개 영상 다운로드 시작", count)
                        } else {
                            "선택된 영상 없음".to_string()
                        }
                    } else {
                        "분석 필요".to_string()
                    };

                    // 분석이 완료된 상태에서만 버튼 활성화
                    if self.playlist_info.is_some() {
                         if ui.button(btn_text).clicked() {
                            if let Err(e) = self.start_download() {
                                self.error_msg = Some(e);
                            }
                        }
                    }
                }
                AppState::Downloading => {
                    ui.label(format!("다운로드 중 ({}/{}):", self.current_download_idx + 1, self.download_queue.len()));
                    if self.current_download_idx < self.download_queue.len() {
                        ui.label(&self.download_queue[self.current_download_idx].title);
                    }
                    ui.add_space(5.0);
                    ui.label(&self.progress_text);
                    ui.add_space(2.0);
                    ui.add(egui::ProgressBar::new(self.progress as f32).animate(true));

                    ui.add_space(5.0);
                    if ui.button("다운로드 중지").clicked() {
                        self.stop_download();
                    }
                }
                AppState::Finished => {
                    ui.label("모든 작업이 완료되었습니다!");
                    ui.horizontal(|ui| {
                        if ui.button("저장 폴더 열기").clicked() {
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(&self.download_dir).spawn();
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("explorer").arg(&self.download_dir).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&self.download_dir).spawn();
                        }

                        if ui.button("목록으로").clicked() {
                            self.state = AppState::Ready;
                            self.current_download_idx = 0;
                            self.progress = 0.0;
                        }
                    });
                }
                _ => {}
            }
             ui.add_space(5.0);
        });

        // 3. Central Panel (리스트)
        egui::CentralPanel::default().show(ctx, |ui| {
             if let Some(info) = &mut self.playlist_info {
                ui.heading(&info.title);
                
                if info.is_playlist {
                     ui.horizontal(|ui| {
                         ui.label(format!("총 {}개의 영상", info.entries.len()));
                         if ui.button("전체 선택").clicked() {
                             for entry in &mut info.entries { entry.selected = true; }
                         }
                         if ui.button("전체 해제").clicked() {
                             for entry in &mut info.entries { entry.selected = false; }
                         }
                     });
                     ui.separator();
                }

                // 스크롤 영역 (최대 높이 제한 제거)
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if info.is_playlist {
                        for (idx, entry) in info.entries.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut entry.selected, "");
                                
                                // 썸네일
                                if let Some(thumb_url) = &entry.thumbnail {
                                    ui.add(egui::Image::from_uri(thumb_url).max_height(50.0).corner_radius(5.0));
                                }

                                ui.vertical(|ui| {
                                    ui.label(format!("{}. {}", idx + 1, entry.title));
                                    ui.label(egui::RichText::new(entry.format_duration()).weak());
                                });
                            });
                            ui.separator();
                        }
                    } else {
                         // 단일 영상도 동일한 리스트 형태로 표시
                        if let Some(entry) = info.entries.first_mut() {
                             ui.horizontal(|ui| {
                                // 단일 영상은 체크박스 굳이 필요 없지만 일관성 유지 or 숨김
                                // ui.checkbox(&mut entry.selected, ""); 
                                
                                if let Some(thumb_url) = &entry.thumbnail {
                                     ui.add(egui::Image::from_uri(thumb_url).max_height(100.0).corner_radius(5.0));
                                }
                                ui.vertical(|ui| {
                                    ui.label(format!("제목: {}", entry.title));
                                    ui.label(format!("길이: {}", entry.format_duration()));
                                });
                            });
                        }
                    }
                });
            } else {
                // 정보 없을 때 안내 문구
                if !matches!(self.state, AppState::Analyzing) {
                    ui.vertical_centered(|ui| {
                         ui.add_space(50.0);
                         ui.label("URL을 입력하고 '분석' 버튼을 눌러주세요.");
                    });
                }
            }
        });
        
        // 애니메이션 효과를 위해 지속적 갱신 필요시 (다운로드 중일 때)
        if matches!(self.state, AppState::Downloading) {
             ctx.request_repaint();
        }
    }
}

// Helper traits/impls
impl AppState {
    fn is_input(&self) -> bool {
        matches!(self, AppState::Input)
    }
}

// download_next에서 스레드 생성시 channel 중계 로직 필요
// downloader::download_video의 인자가 Sender<DownloadStatus> 라서
// UiMessage로 감싸주는 래퍼가 필요.