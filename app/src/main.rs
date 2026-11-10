// ferry (Ferry摇摆人) - a simple multi-media converter, Linux-first, deb-packaged.
// Original studied statically: FormatFactory_setup.exe (NSIS 5.22.2.0) whose payload
// is encrypted; parameters follow app/MECHANICS.md (estimated / ffmpeg public defaults).

/// Product icon, shared by the window icon and the sidebar brand mark.
pub const APP_ICON_PNG: &[u8] = include_bytes!("../deb/icon.png");

use eframe::egui;
use std::path::PathBuf;

mod engine;
mod imaging;
mod tools;
mod ui;

use crate::tools as tools_mod;

/// Audio quality tiers (MP3/AAC/M4A bitrates). See MECHANICS.md (estimated).
pub const AUDIO_TIERS: [(&str, u32); 3] = [("高音质", 320), ("标准", 192), ("普通音质", 128)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Video,
    Audio,
    Image,
    Tools,
    Queue,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JoinMode {
    /// concat demuxer with -c copy (fast, same-codec files only)
    JoinCopy,
    /// concat demuxer, re-encode (more compatible)
    Join,
}

/// Which dialog slot a pending file-picker result belongs to. The tools page
/// has several pickers on screen at once, so it records the slot before
/// opening zenity and routes the result when it arrives.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PickSlot {
    #[default]
    None,
    Join,
    MuxVideo,
    MuxAudio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Collision {
    Overwrite,
    Rename,
    Skip,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub output_dir: Option<PathBuf>,
    pub collision: Collision,
    pub audio_quality: usize, // index into AUDIO_TIERS
    /// Requested image output width. 0 means "use the original width".
    #[serde(default)]
    pub image_width: u32,
    /// Requested image output height. 0 means "use the original height".
    #[serde(default)]
    pub image_height: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: None,
            collision: Collision::Overwrite,
            audio_quality: 1,
            image_width: 0,
            image_height: 0,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        // Prefer the new Ferry config, but keep settings from the earlier
        // fffactory build so a rename does not silently reset them.
        for path in [config_path(), legacy_config_path()] {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(s) = serde_json::from_str::<Settings>(&text) {
                    return s;
                }
            }
        }
        Self::default()
    }

    pub fn image_size(&self) -> Option<(u32, u32)> {
        if self.image_width == 0 && self.image_height == 0 {
            None
        } else {
            Some((self.image_width, self.image_height))
        }
    }

    pub fn save(&self) {
        let p = config_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, text);
        }
    }
}

fn config_path() -> PathBuf {
    crate::dirs_config_dir().join("ferry.json")
}

fn legacy_config_path() -> PathBuf {
    config_dir("fffactory").join("fffactory.json")
}

pub fn dirs_config_dir() -> PathBuf {
    config_dir("ferry")
}

fn config_dir(app: &str) -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        if !x.is_empty() {
            return PathBuf::from(x).join(app);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".config").join(app);
        }
    }
    PathBuf::from(".")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobState {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobKind {
    Convert,
    Image,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    pub source: PathBuf,
    pub target: String, // "mp4", "jpg", ... without dot
    pub target_path: PathBuf,
    /// Snapshot of the requested image output size, taken when the job was
    /// added so later settings changes do not alter queued work.
    pub resize: Option<(u32, u32)>,
    pub state: JobState,
    pub note: String,
}

pub struct App {
    pub page: Page,
    pub settings: Settings,
    pub video_files: Vec<PathBuf>,
    pub audio_files: Vec<PathBuf>,
    pub image_files: Vec<PathBuf>,
    pub video_target: String,
    pub audio_target: String,
    pub image_target: String,
    pub join_files: Vec<PathBuf>,
    pub join_mode: JoinMode,
    pub mux_video: Option<PathBuf>,
    pub mux_audio: Option<PathBuf>,
    pub pick_slot: PickSlot,
    pub jobs: Vec<Job>,
    pub next_id: u64,
    pub status: String,
    pub ffmpeg_found: Option<String>,
    pub shot_path: Option<PathBuf>,
    pub shot_frames_left: u32,
    pub shot_taken: bool,
    pub brand_icon: Option<egui::TextureHandle>,
    pub window_icon_applied: bool,
    pub engine: Option<engine::Engine>,
}

impl Default for App {
    fn default() -> Self {
        let settings = Settings::load();
        Self {
            page: Page::Video,
            settings,
            video_files: Vec::new(),
            audio_files: Vec::new(),
            image_files: Vec::new(),
            video_target: "mp4".into(),
            audio_target: "mp3".into(),
            image_target: "jpg".into(),
            join_files: Vec::new(),
            join_mode: JoinMode::JoinCopy,
            mux_video: None,
            mux_audio: None,
            pick_slot: PickSlot::None,
            jobs: Vec::new(),
            next_id: 1,
            status: String::new(),
            ffmpeg_found: None,
            shot_path: None,
            shot_frames_left: 0,
            shot_taken: false,
            brand_icon: None,
            window_icon_applied: false,
            engine: None,
        }
    }
}

impl App {
    fn ffmpeg_hint(&mut self) {
        if self.ffmpeg_found.is_none() && !tools::ffmpeg_probe_started() {
            tools::start_ffmpeg_probe();
        }
        if let Some(v) = tools::take_ffmpeg_probe_result() {
            self.ffmpeg_found = Some(v.unwrap_or_else(|e| e));
        }
    }

    pub fn add_jobs(&mut self, sources: &[PathBuf], target: &str, kind: JobKind) {
        let resize = if kind == JobKind::Image && target != "ico" {
            self.settings.image_size()
        } else {
            None
        };
        for src in sources {
            let name = src
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "output".into());
            let out_name = format!("{name}_{target}.{target}");
            let dir = self
                .settings
                .output_dir
                .clone()
                .unwrap_or_else(|| src.parent().map(|p| p.to_path_buf()).unwrap_or_default());
            let target_path = dir.join(&out_name);
            self.jobs.push(Job {
                id: self.next_id,
                kind,
                source: src.clone(),
                target: target.to_string(),
                target_path,
                resize,
                state: JobState::Pending,
                note: String::new(),
            });
            self.next_id += 1;
        }
        self.status = format!("已加入 {} 个任务", sources.len());
    }

    pub fn drop_finished(&mut self) {
        self.jobs.retain(|j| {
            !matches!(
                j.state,
                JobState::Done | JobState::Failed | JobState::Skipped
            )
        });
    }
}

/// Headless smoke test mode: convert one file with the production engine and
/// exit. Used by packaging/CI checks (`ferry --selftest <src> <target> <dst>`).
fn selftest(source: &str, target: &str, dest: &str, resize: Option<(u32, u32)>) -> i32 {
    let src = PathBuf::from(source);
    let dst = PathBuf::from(dest);
    const IMAGE_TARGETS: [&str; 9] = [
        "jpg", "png", "webp", "bmp", "gif", "tiff", "ico", "tga", "jpeg",
    ];
    let (ok, note) = if target == "join" {
        // join the single source into dst (exercises the concat path end to end)
        match crate::engine::run_join(&[src], &dst, true) {
            Ok(_) => (true, "完成".into()),
            Err(e) => (false, e),
        }
    } else if IMAGE_TARGETS.contains(&target) {
        let r = imaging::convert(&src, target, &dst, resize);
        (r.ok, r.note)
    } else {
        let quality = AUDIO_TIERS[1].1;
        engine::run_job(&src, target, &dst, quality)
    };
    println!(
        "selftest {target}: {} {note}",
        if ok { "OK" } else { "FAIL" }
    );
    if ok {
        0
    } else {
        1
    }
}

fn main() -> eframe::Result<()> {
    // headless modes first (no GUI)
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() >= 5 && argv[1] == "--selftest" {
        let resize = argv.get(5).and_then(|v| {
            let (w, h) = v.split_once(['x', 'X'])?;
            Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))
        });
        std::process::exit(selftest(&argv[2], &argv[3], &argv[4], resize));
    }
    let mut shot_path: Option<PathBuf> = None;
    let mut start_page: Option<Page> = None;
    let mut start_size: Option<(f32, f32)> = None;
    let mut args = argv.into_iter().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            if let Some(p) = args.next() {
                shot_path = Some(PathBuf::from(p));
            }
        } else if a == "--size" {
            if let Some(v) = args.next() {
                let parts: Vec<&str> = v.split(['x', 'X']).collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        start_size = Some((w, h));
                    }
                }
            }
        } else if a == "--page" {
            if let Some(p) = args.next() {
                start_page = match p.as_str() {
                    "video" => Some(Page::Video),
                    "audio" => Some(Page::Audio),
                    "image" => Some(Page::Image),
                    "tools" => Some(Page::Tools),
                    "queue" => Some(Page::Queue),
                    "settings" => Some(Page::Settings),
                    _ => start_page,
                };
            }
        }
    }

    let (win_w, win_h) = start_size.unwrap_or((1060.0, 660.0));
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([win_w, win_h])
        .with_min_inner_size([880.0, 560.0])
        .with_title("Ferry摇摆人")
        .with_app_id("ferry");
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ferry",
        options,
        Box::new(move |cc| {
            let mut app = App::default();
            if let Some(page) = start_page {
                app.page = page;
            }
            app.shot_path = shot_path;
            if app.shot_path.is_some() {
                app.shot_frames_left = 120;
            }
            app.ffmpeg_hint();
            install_fonts(&cc.egui_ctx);
            app.brand_icon = load_brand_texture(&cc.egui_ctx);
            Box::new(app)
        }),
    )
}

fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn load_brand_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture("ferry-brand", color, egui::TextureOptions::LINEAR))
}

fn apply_window_icon(frame: &eframe::Frame) -> bool {
    #[cfg(target_os = "linux")]
    {
        apply_linux_window_icon(frame)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = frame;
        true
    }
}

/// eframe 0.27 sets the window icon on Windows and macOS, but not on Linux.
/// Set `_NET_WM_ICON` directly on the X11 window so panels and taskbars show
/// the product mark even when the app is launched straight from a build.
/// Wayland compositors use the `app_id` plus the installed .desktop file.
#[cfg(target_os = "linux")]
fn apply_linux_window_icon(frame: &eframe::Frame) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = frame.window_handle() else {
        return false; // window not created yet, retry next frame
    };
    let window = match handle.as_raw() {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        RawWindowHandle::Wayland(_) => return true, // app_id + .desktop handles it
        _ => return true,
    };
    if let Err(e) = set_net_wm_icon(window) {
        eprintln!("taskbar icon: {e}");
    }
    true
}

#[cfg(target_os = "linux")]
fn set_net_wm_icon(window: u32) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

    let (conn, _screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let atom = conn
        .intern_atom(false, b"_NET_WM_ICON")
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .atom;

    let src = image::load_from_memory(APP_ICON_PNG).map_err(|e| e.to_string())?;
    let mut data: Vec<u32> = Vec::new();
    // A few sizes make the panel and window switcher crisp at common scales.
    for size in [128_u32, 64, 48, 32, 24, 16] {
        let icon = src
            .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
        data.push(size);
        data.push(size);
        for pixel in icon.pixels() {
            let [r, g, b, a] = pixel.0;
            data.push(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32));
        }
    }

    conn.change_property32(PropMode::REPLACE, window, atom, AtomEnum::CARDINAL, &data)
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())
}

fn install_fonts(ctx: &egui::Context) {
    // Type direction: Source Han Serif for display headings, Source Han Sans
    // for body text. Both include Latin glyphs, so the UI stops rendering in
    // egui's stock Ubuntu face while keeping CJK coverage.
    let mut fonts = egui::FontDefinitions::default();

    let body: [(&str, &str); 4] = [
        (
            "body-sans",
            "/usr/share/fonts/opentype/source-han-cjk/SourceHanSansSC-Regular.otf",
        ),
        (
            "body-noto",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ),
        ("body-wqy", "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc"),
        (
            "body-dejavu",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ),
    ];
    let display: [(&str, &str); 3] = [
        (
            "display-serif",
            "/usr/share/fonts/opentype/source-han-cjk/SourceHanSerifSC-Medium.otf",
        ),
        (
            "display-noto-serif",
            "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        ),
        (
            "display-sans",
            "/usr/share/fonts/opentype/source-han-cjk/SourceHanSansSC-Medium.otf",
        ),
    ];
    let mono: [(&str, &str); 2] = [
        (
            "mono-han",
            "/usr/share/fonts/opentype/source-han-cjk/SourceHanMono.ttc",
        ),
        (
            "mono-dejavu",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ),
    ];

    let mut body_names = Vec::new();
    for (name, path) in body {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), egui::FontData::from_owned(bytes));
            body_names.push(name.to_string());
        }
    }

    // Body faces first in the proportional family, then egui's built-ins as
    // a glyph fallback.
    for name in body_names.iter().rev() {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name.clone());
    }

    let mut display_names = Vec::new();
    for (name, path) in display {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), egui::FontData::from_owned(bytes));
            display_names.push(name.to_string());
        }
    }
    let display_family = egui::FontFamily::Name("display".into());
    let proportional_fallback = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let family = fonts.families.entry(display_family).or_default();
    for name in &display_names {
        family.push(name.clone());
    }
    for name in &body_names {
        family.push(name.clone());
    }
    // If none of the chosen display faces are installed, keep the UI usable
    // with egui's defaults instead of resolving to an empty family.
    family.extend(proportional_fallback);

    let mut mono_names = Vec::new();
    for (name, path) in mono {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_string(), egui::FontData::from_owned(bytes));
            mono_names.push(name.to_string());
        }
    }
    for name in mono_names.iter().rev() {
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, name.clone());
    }
    for name in &body_names {
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(name.clone());
    }

    ctx.set_fonts(fonts);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.window_icon_applied {
            self.window_icon_applied = apply_window_icon(frame);
        }
        self.ffmpeg_hint();

        // progress conversions; repaint until the queue drains
        engine::pump(ctx, self);

        // tool results (join/mux) land in the status line
        for (label, ok, note) in ui::take_tool_results() {
            let mark = if ok { "✓" } else { "✗" };
            self.status = format!("{mark} {label}: {note}");
        }

        if self.shot_frames_left > 0 {
            self.shot_frames_left -= 1;
            if self.shot_frames_left == 0 {
                // request a screenshot; the reply arrives as an Event next frame
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            ctx.request_repaint();
        }

        // look for the screenshot reply in raw events
        let events = ctx.input(|i| i.raw.events.clone());
        for ev in events {
            if let egui::Event::Screenshot { image, .. } = ev {
                if let Some(p) = self.shot_path.clone() {
                    if !self.shot_taken {
                        self.shot_taken = true;
                        let img = image::RgbaImage::from_raw(
                            image.width() as u32,
                            image.height() as u32,
                            image.pixels.iter().flat_map(|c| c.to_array()).collect(),
                        )
                        .expect("screenshot buffer");
                        let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                        let _ = std::fs::create_dir_all(dir);
                        match img.save(&p) {
                            Ok(_) => println!("screenshot saved: {}", p.display()),
                            Err(e) => eprintln!("screenshot save failed: {e}"),
                        }
                        std::process::exit(0);
                    }
                }
            }
        }

        // keep animating while a file picker is open
        if tools_mod::picker_busy() {
            ctx.request_repaint();
        }

        ui::draw(ctx, self);
    }
}
