#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rust_yt::playlist::{fetch_playlist_info, PlaylistInfo, VideoEntry};
use rust_yt::downloader::{download_video, DownloadConfig, DownloadFormat, DownloadStatus};
use rust_yt::config::AppConfig;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::path::PathBuf;

rust_i18n::i18n!("locales");

fn main() -> eframe::Result<()> {
    // 폰트 설정 (임베디드 폰트)
    // 윈도우/리눅스 모두에서 한글 깨짐을 방지하기 위해 폰트를 바이너리에 포함
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_resizable(true)
            .with_icon(load_icon()),
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

fn load_icon() -> eframe::egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/icon.ico");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    
    eframe::egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Font Data Loaded
    fonts.font_data.insert(
        "NanumGothic".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NanumGothic.ttf"
        ))),
    );
    fonts.font_data.insert(
        "NotoSansJP".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansCJKjp-Regular.otf"
        ))),
    );
    fonts.font_data.insert(
        "NotoSansSC".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansCJKsc-Regular.otf"
        ))),
    );

    // 2. Proportional Priority: Nanum > JP > SC > Default
    // `insert(0, ...)` prepends. To get A, B, C order, we can insert C, then B, then A.
    // Or insert A at 0, B at 1, C at 2.
    // Default likely has stuff.
    
    let proportional = fonts.families.entry(egui::FontFamily::Proportional).or_default();
    proportional.insert(0, "NanumGothic".to_owned());
    proportional.insert(1, "NotoSansJP".to_owned());
    proportional.insert(2, "NotoSansSC".to_owned());

    // 3. Monospace Priority: Nanum > JP > SC > Default
    let monospace = fonts.families.entry(egui::FontFamily::Monospace).or_default();
    monospace.insert(0, "NanumGothic".to_owned());
    monospace.insert(1, "NotoSansJP".to_owned());
    monospace.insert(2, "NotoSansSC".to_owned());

    ctx.set_fonts(fonts);
}

#[derive(Debug)]
enum AppState {
    Initializing, // [NEW] 초기화 (다운로드 등)
    SetPath, // [NEW] 초기 경로 설정
    Input,
    Analyzing,
    Ready,
    Downloading,
    Finished,
}

struct MyApp {
    download_dir: PathBuf, // 저장 경로
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
    stop_tx: Option<Sender<()>>,
    
    // 초기화 상태 표시용
    init_status: String,
    init_progress: f32,
    
    // 설정 저장 시 경로 설정 건너뛰기
    skip_set_path: bool,
}

enum UiMessage {
    InitStatus(rust_yt::initializer::InitStatus),
    AnalysisDone(Result<PlaylistInfo, String>),
    DownloadProgress(DownloadStatus),
}

    impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = channel();

        // 저장된 설정 로드
        let saved_config = AppConfig::load();
        
        // 언어 설정 적용
        let locale = if saved_config.language == "auto" {
            sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
        } else {
            saved_config.language.clone()
        };
        rust_i18n::set_locale(&locale);

        let initial_dir = saved_config.download_dir.clone().unwrap_or_default();
        let initial_format = AppConfig::string_to_format(&saved_config.format);
        
        // 저장된 경로가 있으면 SetPath 단계 건너뛰기
        let _initial_state = if saved_config.download_dir.is_some() {
            AppState::Input
        } else {
            AppState::Initializing
        };

        // [초기화 스레드 시작]
        let tx_clone = tx.clone();
        let has_saved_path = saved_config.download_dir.is_some();
        thread::spawn(move || {
            let (init_tx, init_rx) = channel();
            
            // 실제 초기화 작업 수행 (별도 스레드)
            thread::spawn(move || {
                rust_yt::initializer::init_dependencies(init_tx);
            });

            // UI로 상태 전달
            while let Ok(status) = init_rx.recv() {
                // 저장된 경로가 있으면 Completed 시 Input으로 직행
                let modified_status = if has_saved_path {
                    if let rust_yt::initializer::InitStatus::Completed = &status {
                        // Completed 상태는 그대로 전달 (이미 initial_state가 Input임)
                    }
                    status
                } else {
                    status
                };
                
                if tx_clone.send(UiMessage::InitStatus(modified_status)).is_err() {
                    break;
                }
            }
        });

        Self {
            download_dir: initial_dir,
            url: String::new(),
            format: initial_format,
            state: if saved_config.download_dir.is_some() { 
                AppState::Initializing // 초기화 후 Input으로
            } else {
                AppState::Initializing
            },
            playlist_info: None,
            error_msg: None,
            download_queue: Vec::new(),
            current_download_idx: 0,
            progress: 0.0,
            progress_text: String::new(),
            tx_ui: tx,
            rx_ui: rx,
            stop_tx: None,
            init_status: rust_i18n::t!("initialization.preparing").to_string(),
            init_progress: 0.0,
            skip_set_path: saved_config.download_dir.is_some(),
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
        let info = self.playlist_info.as_ref().ok_or(rust_i18n::t!("main.need_analysis").to_string())?;
        
        // 선택된 영상만 필터링
        self.download_queue = info.entries.iter()
            .filter(|e| e.selected)
            .cloned()
            .collect();
            
        if self.download_queue.is_empty() {
            return Err(rust_i18n::t!("main.no_selection").to_string());
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
        self.progress_text = rust_i18n::t!("main.download_stopped").to_string();
    }

    fn download_next(&mut self) {
        if self.current_download_idx >= self.download_queue.len() {
            self.state = AppState::Finished;
            self.progress_text = rust_i18n::t!("main.all_completed").to_string();
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
        self.progress_text = rust_i18n::t!("main.preparing_video", title = video.title).to_string();
        
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

    fn save_config(&self) {
        let config = AppConfig {
            download_dir: Some(self.download_dir.clone()),
            format: AppConfig::format_to_string(&self.format),
            audio_quality: "320K".to_string(),
            language: rust_i18n::locale().to_string(),
        };
        let _ = config.save();
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 메시지 처리
        while let Ok(msg) = self.rx_ui.try_recv() {
            match msg {
                UiMessage::InitStatus(status) => {
                    match status {
                        rust_yt::initializer::InitStatus::Starting(msg) => {
                            self.init_status = msg;
                            self.init_progress = 0.0;
                        }
                        rust_yt::initializer::InitStatus::Downloading(p, msg) => {
                            self.init_progress = (p / 100.0) as f32;
                            self.init_status = rust_i18n::t!("initialization.downloading", file = msg, percent = format!("{:.1}", p)).to_string();
                        }
                        rust_yt::initializer::InitStatus::Extracting(msg) => {
                            self.init_status = msg;
                            self.init_progress = 1.0; // 인디터미네이트로 쓸 수도 있음
                        }
                        rust_yt::initializer::InitStatus::Completed => {
                            if self.skip_set_path {
                                self.state = AppState::Input;
                            } else {
                                self.state = AppState::SetPath;
                            }
                        }
                        rust_yt::initializer::InitStatus::Failed(e) => {
                            self.error_msg = Some(format!("초기화 실패: {}", e));
                            // 실패해도 일단 진행? 아니면 재시도? 일단 진행시켜서 수동 설정 유도하거나 에러 표시
                            self.state = AppState::SetPath; 
                        }
                    }
                }
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
                            self.progress_text = rust_i18n::t!("main.converting").to_string();
                        }
                        DownloadStatus::Completed(_) => {
                            self.current_download_idx += 1;
                            self.download_next();
                        }
                        DownloadStatus::Failed(e) => {
                            if self.progress_text == rust_i18n::t!("main.download_stopped").to_string() {
                                self.state = AppState::Ready;
                                self.progress_text = rust_i18n::t!("main.download_stopped").to_string();
                            } else {
                                self.progress_text = format!("오류: {}", e);
                                self.error_msg = Some(rust_i18n::t!("main.download_paused", error = e).to_string());
                                self.state = AppState::Ready;
                            }
                            self.stop_tx = None;
                        }
                        DownloadStatus::Stopped => {
                            self.state = AppState::Ready;
                            self.progress_text = rust_i18n::t!("main.download_stopped").to_string();
                            self.stop_tx = None;
                        }
                    }
                }
            }
        }

        // -1. 초기화 화면
        if matches!(self.state, AppState::Initializing) {
             // 렌더링 루프 초기에 한 번만 실행되도록 플래그를 쓰거나, 
             // 생성자에서 스레드를 띄우는 게 낫지만 eframe 특성상 여기서 띄우기도 가능.
             // 하지만 매 프레임 실행되면 안됨.
             // MyApp 구조체에 `init_started` 필드를 두거나, 
             // tx/rx가 있으므로 생성자에서 그냥 띄우는게 낫다. 
             // -> 생성자에서는 self.tx_ui를 클론해서 넘겨주기가 까다로울 수 있음 (Channel은 되지만)
             // 여기서는 간단히 "한 번만 실행" 로직을 넣기보다,
             // MyApp::default()가 호출될 때 thread를 띄우는게 정석.
             // 하지만 MyApp::default는 &self가 아니라서 필드 접근 불가.
             // setup_custom_fonts 호출하는 closure 안에서 
             // MyApp 생성 후, 거기서 띄우는 방법. 
             // 일단 여기서는 꼼수로... static flag나 Option check?
             // 아님 그냥 별도 함수 start_init() 만들어서 생성자에서 호출? 
             // -> 생성자에서 호출하자.
             
             egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading(rust_i18n::t!("initialization.title"));
                    ui.add_space(20.0);
                    ui.spinner();
                    ui.add_space(20.0);
                    ui.label(&self.init_status);
                    ui.add_space(10.0);
                    ui.add(egui::ProgressBar::new(self.init_progress).animate(true));
                });
            });
            return;
        }

        // 0. 초기 경로 설정 화면
        if matches!(self.state, AppState::SetPath) {
             egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading(rust_i18n::t!("main.title"));
                    ui.add_space(50.0);
                    ui.label(rust_i18n::t!("main.select_folder_msg"));
                    ui.add_space(20.0);
                    if ui.button(rust_i18n::t!("main.select_folder_btn")).clicked() {
                         if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.download_dir = path.clone();
                            self.state = AppState::Input;
                            // 설정 저장
                            self.save_config();
                        }
                    }
                });
            });
            return;
        }

        // 1. Top Panel (설정 및 입력)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.heading(rust_i18n::t!("main.title"));
            ui.add_space(5.0);

            // [NEW] 언어 선택
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("main.language_label"));
                let current_locale = rust_i18n::locale().to_string();
                let mut selected_locale = current_locale.clone();
                
                egui::ComboBox::from_id_salt("lang_combo")
                    .selected_text(match selected_locale.as_str() {
                        "en" => "English",
                        "ko" => "한국어",
                        "ja" => "日本語",
                        "zh-CN" => "中文 (简体)",
                        _ => "English", // Default fallback
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected_locale, "en".to_string(), "English");
                        ui.selectable_value(&mut selected_locale, "ko".to_string(), "한국어");
                        ui.selectable_value(&mut selected_locale, "ja".to_string(), "日本語");
                        ui.selectable_value(&mut selected_locale, "zh-CN".to_string(), "中文 (简体)");
                    });

                if selected_locale != current_locale {
                    rust_i18n::set_locale(&selected_locale);
                     // 설정 저장
                    self.save_config();
                }
            });
            
            ui.separator();

            // 경로 등
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("main.save_path", path = self.download_dir.display()));
                if ui.button(rust_i18n::t!("main.change_btn")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.download_dir = path.clone();
                        // 설정 저장
                        self.save_config();
                    }
                }
            });
            ui.separator();

            // URL 입력
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("main.url_label"));
                let text_edit = ui.text_edit_singleline(&mut self.url);
                if self.state.is_input() || matches!(self.state, AppState::Ready | AppState::Finished) {
                    if ui.button(rust_i18n::t!("main.analyze_btn")).clicked() || (text_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))) {
                        if !self.url.trim().is_empty() {
                            self.start_analysis();
                        }
                    }
                }
            });

            ui.add_space(5.0);

            // 형식 선택
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("main.format_label"));
                let prev_format = self.format.clone();
                egui::ComboBox::from_id_salt("format_combo")
                    .selected_text(match self.format {
                        DownloadFormat::Mp3 => rust_i18n::t!("formats.audio_mp3"),
                        DownloadFormat::Wav => rust_i18n::t!("formats.audio_wav"),
                        DownloadFormat::M4a => rust_i18n::t!("formats.audio_m4a"),
                        DownloadFormat::Flac => rust_i18n::t!("formats.audio_flac"),
                        DownloadFormat::Mp4 => rust_i18n::t!("formats.video_mp4"),
                        DownloadFormat::Webm => rust_i18n::t!("formats.video_webm"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.format, DownloadFormat::Mp3, rust_i18n::t!("formats.audio_mp3"));
                        ui.selectable_value(&mut self.format, DownloadFormat::Wav, rust_i18n::t!("formats.audio_wav"));
                        ui.selectable_value(&mut self.format, DownloadFormat::M4a, rust_i18n::t!("formats.audio_m4a"));
                        ui.selectable_value(&mut self.format, DownloadFormat::Flac, rust_i18n::t!("formats.audio_flac"));
                        ui.separator();
                        ui.selectable_value(&mut self.format, DownloadFormat::Mp4, rust_i18n::t!("formats.video_mp4"));
                        ui.selectable_value(&mut self.format, DownloadFormat::Webm, rust_i18n::t!("formats.video_webm"));
                    });
                
                // 포맷 변경 시 설정 저장
                if prev_format != self.format {
                    self.save_config();
                }
            });

             // 로딩 상태 (Top Panel에 표시)
            if matches!(self.state, AppState::Analyzing) {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(rust_i18n::t!("main.analyzing_msg"));
                });
            }
            
             ui.add_space(5.0);
        });

        // 2. Bottom Panel (액션, 상태, 프로그레스)
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            
            // 에러 메시지
            if let Some(err) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, rust_i18n::t!("main.error_prefix", msg = err));
                ui.separator();
            }

            // 다운로드 컨트롤
            match self.state {
                AppState::Ready => {
                    let btn_text = if let Some(info) = &self.playlist_info {
                        let count = info.entries.iter().filter(|e| e.selected).count();
                        if count > 0 {
                            rust_i18n::t!("main.download_start_count", count = count)
                        } else {
                            rust_i18n::t!("main.no_selection")
                        }
                    } else {
                        rust_i18n::t!("main.need_analysis")
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
                    ui.label(rust_i18n::t!("main.downloading_status", current = self.current_download_idx + 1, total = self.download_queue.len()));
                    if self.current_download_idx < self.download_queue.len() {
                        ui.label(&self.download_queue[self.current_download_idx].title);
                    }
                    ui.add_space(5.0);
                    ui.label(&self.progress_text);
                    ui.add_space(2.0);
                    ui.add(egui::ProgressBar::new(self.progress as f32).animate(true));

                    ui.add_space(5.0);
                    if ui.button(rust_i18n::t!("main.stop_download_btn")).clicked() {
                        self.stop_download();
                    }
                }
                AppState::Finished => {
                    ui.label(rust_i18n::t!("main.all_completed"));
                    ui.horizontal(|ui| {
                        if ui.button(rust_i18n::t!("main.open_folder_btn")).clicked() {
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(&self.download_dir).spawn();
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("explorer").arg(&self.download_dir).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&self.download_dir).spawn();
                        }

                        if ui.button(rust_i18n::t!("main.back_to_list_btn")).clicked() {
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
                         ui.label(rust_i18n::t!("main.total_videos", count = info.entries.len()));
                         if ui.button(rust_i18n::t!("main.select_all")).clicked() {
                             for entry in &mut info.entries { entry.selected = true; }
                         }
                         if ui.button(rust_i18n::t!("main.deselect_all")).clicked() {
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
                                    ui.label(rust_i18n::t!("main.video_title", title = entry.title));
                                    ui.label(rust_i18n::t!("main.video_duration", duration = entry.format_duration()));
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
                         ui.label(rust_i18n::t!("main.input_url_hint"));
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