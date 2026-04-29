#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use rust_yt::config::AppConfig;
use rust_yt::downloader::{DownloadConfig, DownloadFormat, DownloadStatus, download_video};
use rust_yt::playlist::{PlaylistInfo, VideoEntry, fetch_playlist_info_with_channel};
use rust_yt::ytdlp::YtDlpChannel;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

rust_i18n::i18n!("locales");

fn main() -> eframe::Result<()> {
    // 임베디드 폰트 설정
    // Windows, macOS, Linux에서 한글이 깨지지 않도록 폰트를 바이너리에 포함합니다.

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([480.0, 360.0])
            .with_resizable(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "YouTube Downloader",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            setup_app_style(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx); // 이미지 로더 설치
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

    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional.insert(0, "NanumGothic".to_owned());
    proportional.insert(1, "NotoSansJP".to_owned());
    proportional.insert(2, "NotoSansSC".to_owned());

    // 3. Monospace Priority: Nanum > JP > SC > Default
    let monospace = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    monospace.insert(0, "NanumGothic".to_owned());
    monospace.insert(1, "NotoSansJP".to_owned());
    monospace.insert(2, "NotoSansSC".to_owned());

    ctx.set_fonts(fonts);
}

fn setup_app_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(242, 244, 248);
    visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
    visuals.faint_bg_color = egui::Color32::from_rgb(234, 237, 243);
    visuals.extreme_bg_color = egui::Color32::from_rgb(223, 227, 235);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 122, 255);
    visuals.selection.stroke.color = egui::Color32::WHITE;
    visuals.hyperlink_color = egui::Color32::from_rgb(0, 102, 204);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(216, 220, 228));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 242, 252);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(174, 199, 238));
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.interact_size = egui::vec2(72.0, 30.0);
        style.spacing.window_margin = egui::Margin::same(10);
    });
}

#[derive(Debug)]
enum AppState {
    Initializing, // 초기화 및 의존성 준비
    SetPath,      // 초기 다운로드 경로 설정
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
    ytdlp_channel: YtDlpChannel,
    state: AppState,
    playlist_info: Option<PlaylistInfo>,
    error_msg: Option<String>,

    // 다운로드 상태
    download_queue: Vec<VideoEntry>,
    current_download_idx: usize,
    progress: f64,
    progress_text: String,
    download_log: Vec<String>,

    // 비동기 통신
    tx_ui: Sender<UiMessage>,
    rx_ui: Receiver<UiMessage>,
    stop_tx: Option<Sender<()>>,

    // 초기화 상태 표시
    init_status: String,
    init_progress: f32,

    // 설정 저장 시 경로 설정 단계 건너뛰기
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
        let initial_ytdlp_channel = saved_config.ytdlp_channel();

        // 저장된 경로가 있으면 SetPath 단계 건너뛰기
        let _initial_state = if saved_config.download_dir.is_some() {
            AppState::Input
        } else {
            AppState::Initializing
        };

        // 초기화 스레드 시작
        let tx_clone = tx.clone();
        let has_saved_path = saved_config.download_dir.is_some();
        let init_ytdlp_channel = initial_ytdlp_channel;
        thread::spawn(move || {
            let (init_tx, init_rx) = channel();

            // 실제 초기화 작업 수행
            thread::spawn(move || {
                rust_yt::initializer::init_dependencies(init_tx, init_ytdlp_channel);
            });

            // UI로 상태 전달
            while let Ok(status) = init_rx.recv() {
                // 저장된 경로가 있으면 Completed 시 Input으로 직행
                let modified_status = if has_saved_path {
                    if let rust_yt::initializer::InitStatus::Completed = &status {
                        // Completed 상태는 그대로 전달
                    }
                    status
                } else {
                    status
                };

                if tx_clone
                    .send(UiMessage::InitStatus(modified_status))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            download_dir: initial_dir,
            url: String::new(),
            format: initial_format,
            ytdlp_channel: initial_ytdlp_channel,
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
            download_log: Vec::new(),
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
        let ytdlp_channel = self.ytdlp_channel;
        let tx = self.tx_ui.clone();

        self.state = AppState::Analyzing;
        self.error_msg = None;

        thread::spawn(move || {
            let result = fetch_playlist_info_with_channel(&url, ytdlp_channel);
            let _ = tx.send(UiMessage::AnalysisDone(result));
        });
    }

    fn start_download(&mut self) -> Result<(), String> {
        let info = self
            .playlist_info
            .as_ref()
            .ok_or(rust_i18n::t!("main.need_analysis").to_string())?;

        // 선택된 영상만 필터링
        self.download_queue = info
            .entries
            .iter()
            .filter(|e| e.selected)
            .cloned()
            .collect();

        if self.download_queue.is_empty() {
            return Err(rust_i18n::t!("main.no_selection").to_string());
        }

        self.current_download_idx = 0;
        self.download_log.clear();
        self.state = AppState::Downloading;
        self.download_next();
        Ok(())
    }

    fn stop_download(&mut self) {
        if let Some(tx) = &self.stop_tx {
            let _ = tx.send(());
        }
        // 중지 신호를 보낸 뒤 다운로드 스레드가 상태 메시지를 보낼 때까지 기다립니다.
        // UI 반응성을 위해 화면 상태는 즉시 갱신합니다.
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
            output_dir: self.download_dir.clone(), // 선택한 저장 경로 사용
        };

        // UI 초기화
        self.progress = 0.0;
        self.progress_text = rust_i18n::t!("main.preparing_video", title = video.title).to_string();
        self.push_download_log(self.progress_text.clone());

        // 중지 채널 생성
        let (stop_tx, stop_rx) = channel();
        self.stop_tx = Some(stop_tx);

        thread::spawn(move || {
            let (tx_internal, rx_internal) = channel();

            // 별도 스레드에서 다운로드 실행
            let config_clone = config.clone();
            let title_clone = video.title.clone();
            let tx_internal_clone = tx_internal.clone();

            thread::spawn(move || {
                download_video(config_clone, title_clone, tx_internal_clone, stop_rx);
            });

            // 진행 상태 중계
            while let Ok(status) = rx_internal.recv() {
                match tx.send(UiMessage::DownloadProgress(status)) {
                    Ok(_) => {}
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
            ytdlp_channel: self.ytdlp_channel.as_str().to_string(),
        };
        let _ = config.save();
    }

    fn centered_square_uv(image_size: egui::Vec2) -> egui::Rect {
        if image_size.x <= 0.0 || image_size.y <= 0.0 {
            return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        }

        if image_size.x > image_size.y {
            let width_ratio = image_size.y / image_size.x;
            let margin = (1.0 - width_ratio) * 0.5;
            egui::Rect::from_min_max(egui::pos2(margin, 0.0), egui::pos2(1.0 - margin, 1.0))
        } else {
            let height_ratio = image_size.x / image_size.y;
            let margin = (1.0 - height_ratio) * 0.5;
            egui::Rect::from_min_max(egui::pos2(0.0, margin), egui::pos2(1.0, 1.0 - margin))
        }
    }

    fn add_square_thumbnail(ui: &mut egui::Ui, ctx: &egui::Context, thumb_url: &str, size: f32) {
        let thumbnail_size = egui::vec2(size, size);

        let image = match ctx.try_load_texture(
            thumb_url,
            egui::TextureOptions::LINEAR,
            egui::load::SizeHint::from(thumbnail_size),
        ) {
            Ok(egui::load::TexturePoll::Ready { texture }) => {
                let uv = Self::centered_square_uv(texture.size);
                egui::Image::new(texture).uv(uv)
            }
            _ => egui::Image::from_uri(thumb_url),
        };

        ui.add(image.fit_to_exact_size(thumbnail_size).corner_radius(5.0));
    }

    fn push_download_log(&mut self, message: impl Into<String>) {
        const MAX_DOWNLOAD_LOG_LINES: usize = 80;

        let message = message.into();
        if message.trim().is_empty() {
            return;
        }

        self.download_log.push(message);
        if self.download_log.len() > MAX_DOWNLOAD_LOG_LINES {
            let overflow = self.download_log.len() - MAX_DOWNLOAD_LOG_LINES;
            self.download_log.drain(0..overflow);
        }
    }

    fn section_frame() -> egui::Frame {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(198, 205, 218),
            ))
            .corner_radius(8)
            .shadow(egui::Shadow {
                offset: [0, 1],
                blur: 8,
                spread: 0,
                color: egui::Color32::from_black_alpha(18),
            })
            .inner_margin(egui::Margin::symmetric(12, 10))
    }

    fn render_app_header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading(rust_i18n::t!("main.title"));
        ui.add_space(8.0);

        Self::section_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(rust_i18n::t!("main.language_label"));
                let current_locale = rust_i18n::locale().to_string();
                let mut selected_locale = current_locale.clone();

                egui::ComboBox::from_id_salt("lang_combo")
                    .selected_text(match selected_locale.as_str() {
                        "en" => "English",
                        "ko" => "한국어",
                        "ja" => "日本語",
                        "zh-CN" => "中文",
                        _ => "English",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected_locale, "en".to_string(), "English");
                        ui.selectable_value(&mut selected_locale, "ko".to_string(), "한국어");
                        ui.selectable_value(&mut selected_locale, "ja".to_string(), "日本語");
                        ui.selectable_value(&mut selected_locale, "zh-CN".to_string(), "中文");
                    });

                ui.separator();
                ui.label("yt-dlp");
                let previous_channel = self.ytdlp_channel;
                egui::ComboBox::from_id_salt("ytdlp_channel_combo")
                    .selected_text(self.ytdlp_channel.as_str())
                    .show_ui(ui, |ui| {
                        for channel in YtDlpChannel::ALL {
                            ui.selectable_value(&mut self.ytdlp_channel, channel, channel.as_str());
                        }
                    });

                if selected_locale != current_locale {
                    rust_i18n::set_locale(&selected_locale);
                    self.save_config();
                }
                if previous_channel != self.ytdlp_channel {
                    self.save_config();
                }
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::Label::new(rust_i18n::t!(
                        "main.save_path",
                        path = self.download_dir.display()
                    ))
                    .wrap(),
                );
                if ui.button(rust_i18n::t!("main.change_btn")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.download_dir = path.clone();
                        self.save_config();
                    }
                }
            });

            ui.add_space(8.0);
            let compact_url_row = ui.available_width() < 420.0;
            if compact_url_row {
                ui.label(rust_i18n::t!("main.url_label"));
            }
            ui.horizontal_wrapped(|ui| {
                if !compact_url_row {
                    ui.label(rust_i18n::t!("main.url_label"));
                }
                if self.state.is_input()
                    || matches!(self.state, AppState::Ready | AppState::Finished)
                {
                    let edit_width = if compact_url_row {
                        ui.available_width().max(220.0)
                    } else {
                        (ui.available_width() - 96.0).max(220.0)
                    };
                    let text_edit = ui.add_sized(
                        [edit_width, ui.spacing().interact_size.y],
                        egui::TextEdit::singleline(&mut self.url),
                    );
                    if ui.button(rust_i18n::t!("main.analyze_btn")).clicked()
                        || (text_edit.lost_focus()
                            && ctx.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        if !self.url.trim().is_empty() {
                            self.start_analysis();
                        }
                    }
                } else {
                    ui.add_sized(
                        [
                            ui.available_width().max(180.0),
                            ui.spacing().interact_size.y,
                        ],
                        egui::TextEdit::singleline(&mut self.url),
                    );
                }
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
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
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::Mp3,
                            rust_i18n::t!("formats.audio_mp3"),
                        );
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::Wav,
                            rust_i18n::t!("formats.audio_wav"),
                        );
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::M4a,
                            rust_i18n::t!("formats.audio_m4a"),
                        );
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::Flac,
                            rust_i18n::t!("formats.audio_flac"),
                        );
                        ui.separator();
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::Mp4,
                            rust_i18n::t!("formats.video_mp4"),
                        );
                        ui.selectable_value(
                            &mut self.format,
                            DownloadFormat::Webm,
                            rust_i18n::t!("formats.video_webm"),
                        );
                    });

                if prev_format != self.format {
                    self.save_config();
                }
            });

            if matches!(self.state, AppState::Analyzing) {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add(egui::Label::new(rust_i18n::t!("main.analyzing_msg")).wrap());
                });
            }
        });
    }

    fn render_work_area(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        Self::section_frame().show(ui, |ui| {
            if let Some(info) = &mut self.playlist_info {
                ui.heading(&info.title);

                if info.is_playlist {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(rust_i18n::t!(
                            "main.total_videos",
                            count = info.entries.len()
                        ));
                        if ui.button(rust_i18n::t!("main.select_all")).clicked() {
                            for entry in &mut info.entries {
                                entry.selected = true;
                            }
                        }
                        if ui.button(rust_i18n::t!("main.deselect_all")).clicked() {
                            for entry in &mut info.entries {
                                entry.selected = false;
                            }
                        }
                    });
                    ui.add_space(8.0);
                }

                if info.is_playlist {
                    for (idx, entry) in info.entries.iter_mut().enumerate() {
                        Self::video_row_frame().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut entry.selected, "");

                                if let Some(thumb_url) = &entry.thumbnail {
                                    Self::add_square_thumbnail(ui, ctx, thumb_url, 50.0);
                                }

                                ui.vertical(|ui| {
                                    ui.add(
                                        egui::Label::new(format!("{}. {}", idx + 1, entry.title))
                                            .wrap(),
                                    );
                                    ui.label(egui::RichText::new(entry.format_duration()).weak());
                                });
                            });
                        });
                        ui.add_space(6.0);
                    }
                } else if let Some(entry) = info.entries.first_mut() {
                    Self::video_row_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(thumb_url) = &entry.thumbnail {
                                Self::add_square_thumbnail(ui, ctx, thumb_url, 100.0);
                            }
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(rust_i18n::t!(
                                        "main.video_title",
                                        title = entry.title
                                    ))
                                    .wrap(),
                                );
                                ui.add(
                                    egui::Label::new(rust_i18n::t!(
                                        "main.video_duration",
                                        duration = entry.format_duration()
                                    ))
                                    .wrap(),
                                );
                            });
                        });
                    });
                }
            } else if !matches!(self.state, AppState::Analyzing) {
                ui.vertical_centered(|ui| {
                    ui.add_space(14.0);
                    ui.add(egui::Label::new(rust_i18n::t!("main.input_url_hint")).wrap());
                    ui.add_space(14.0);
                });
            }
        });
    }

    fn video_row_frame() -> egui::Frame {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(255, 255, 255))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(207, 214, 226),
            ))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 7))
    }

    fn render_status_area(&mut self, ui: &mut egui::Ui) {
        Self::section_frame().show(ui, |ui| {
            if let Some(err) = &self.error_msg {
                ui.colored_label(
                    egui::Color32::from_rgb(200, 42, 42),
                    rust_i18n::t!("main.error_prefix", msg = err),
                );
                ui.add_space(8.0);
            }

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

                    if self.playlist_info.is_some() {
                        if ui.button(btn_text).clicked() {
                            if let Err(e) = self.start_download() {
                                self.error_msg = Some(e);
                            }
                        }
                    }
                }
                AppState::Downloading => {
                    ui.label(rust_i18n::t!(
                        "main.downloading_status",
                        current = self.current_download_idx + 1,
                        total = self.download_queue.len()
                    ));
                    if self.current_download_idx < self.download_queue.len() {
                        ui.add(
                            egui::Label::new(&self.download_queue[self.current_download_idx].title)
                                .wrap(),
                        );
                    }
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(egui::RichText::new(&self.progress_text).strong()).wrap(),
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::ProgressBar::new(self.progress as f32)
                            .animate(true)
                            .desired_height(8.0)
                            .corner_radius(4),
                    );

                    ui.add_space(10.0);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(250, 251, 253))
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgb(226, 230, 238),
                        ))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            for line in self.download_log.iter().rev().take(12).rev() {
                                ui.add(
                                    egui::Label::new(egui::RichText::new(line).monospace().weak())
                                        .wrap(),
                                );
                            }
                        });

                    ui.add_space(10.0);
                    if ui.button(rust_i18n::t!("main.stop_download_btn")).clicked() {
                        self.stop_download();
                    }
                }
                AppState::Finished => {
                    ui.label(rust_i18n::t!("main.all_completed"));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(rust_i18n::t!("main.open_folder_btn")).clicked() {
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&self.download_dir)
                                .spawn();
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("explorer")
                                .arg(&self.download_dir)
                                .spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open")
                                .arg(&self.download_dir)
                                .spawn();
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
        });
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
                            self.init_status = rust_i18n::t!(
                                "initialization.downloading",
                                file = msg,
                                percent = format!("{:.1}", p)
                            )
                            .to_string();
                        }
                        rust_yt::initializer::InitStatus::Extracting(msg) => {
                            self.init_status = msg;
                            self.init_progress = 1.0; // 인디케이터를 완료 상태로 표시
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
                            // 초기화가 실패해도 오류를 표시하고 입력 화면으로 진행합니다.
                            self.state = AppState::SetPath;
                        }
                    }
                }
                UiMessage::AnalysisDone(result) => match result {
                    Ok(info) => {
                        self.playlist_info = Some(info);
                        self.state = AppState::Ready;
                    }
                    Err(e) => {
                        self.error_msg = Some(e);
                        self.state = AppState::Input;
                    }
                },
                UiMessage::DownloadProgress(status) => match status {
                    DownloadStatus::Starting(msg) => {
                        self.push_download_log(msg.clone());
                        self.progress_text = msg;
                        self.progress = 0.0;
                    }
                    DownloadStatus::Progress(p, speed) => {
                        self.progress = p / 100.0;
                        self.progress_text = format!("{:.1}% ({})", p, speed);
                        self.push_download_log(self.progress_text.clone());
                    }
                    DownloadStatus::Message(msg) => {
                        self.push_download_log(msg.clone());
                        self.progress_text = msg;
                    }
                    DownloadStatus::Converting => {
                        self.progress_text = rust_i18n::t!("main.converting").to_string();
                        self.push_download_log(self.progress_text.clone());
                    }
                    DownloadStatus::Completed(title) => {
                        self.push_download_log(format!("완료: {}", title));
                        self.current_download_idx += 1;
                        self.download_next();
                    }
                    DownloadStatus::Failed(e) => {
                        if self.progress_text == rust_i18n::t!("main.download_stopped").to_string()
                        {
                            self.state = AppState::Ready;
                            self.progress_text = rust_i18n::t!("main.download_stopped").to_string();
                        } else {
                            self.progress_text = format!("?ㅻ쪟: {}", e);
                            self.push_download_log(self.progress_text.clone());
                            self.error_msg =
                                Some(rust_i18n::t!("main.download_paused", error = e).to_string());
                            self.state = AppState::Ready;
                        }
                        self.push_download_log(self.progress_text.clone());
                        self.stop_tx = None;
                    }
                    DownloadStatus::Stopped => {
                        self.state = AppState::Ready;
                        self.progress_text = rust_i18n::t!("main.download_stopped").to_string();
                        self.push_download_log(self.progress_text.clone());
                        self.stop_tx = None;
                    }
                },
            }
        }

        // 초기화 화면
        if matches!(self.state, AppState::Initializing) {
            // 초기화 상태는 생성자에서 시작한 작업 결과를 표시합니다.
            // 화면은 스레드를 다시 시작하지 않고 수신된 상태만 반영합니다.

            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(48.0);
                            ui.heading(rust_i18n::t!("initialization.title"));
                            ui.add_space(14.0);
                            ui.spinner();
                            ui.add_space(14.0);
                            ui.add(egui::Label::new(&self.init_status).wrap());
                            ui.add_space(8.0);
                            ui.add(egui::ProgressBar::new(self.init_progress).animate(true));
                        });
                    });
            });
            return;
        }

        // 초기 경로 설정 화면
        if matches!(self.state, AppState::SetPath) {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(36.0);
                            ui.heading(rust_i18n::t!("main.title"));
                            ui.add_space(24.0);
                            ui.add(
                                egui::Label::new(rust_i18n::t!("main.select_folder_msg")).wrap(),
                            );
                            ui.add_space(14.0);
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
            });
            return;
        }

        // 메인 레이아웃
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    self.render_app_header(ui, ctx);
                    ui.add_space(8.0);
                    self.render_work_area(ui, ctx);
                    ui.add_space(8.0);
                    self.render_status_area(ui);
                    ui.add_space(8.0);
                });
        });

        // 다운로드 중에는 진행률 애니메이션을 위해 지속적으로 갱신합니다.
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

// download_next에서 생성한 스레드와 UI 메시지를 중계합니다.
// downloader::download_video는 Sender<DownloadStatus>를 사용합니다.
// UiMessage로 감싸는 어댑터가 필요합니다.
