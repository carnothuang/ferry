// The UI. One window, a dark ink rail on the left, paper-coloured work
// surfaces on the right. Pages: Video / Audio / Image converters, Tools
// (join & mux), Queue, Settings.
//
// Design direction: a workshop instrument, not a generic SaaS dashboard.
// Ink + paper, one signal teal sampled from the product icon, Source Han
// Serif for display type,
// Source Han Sans for body, hairline rules instead of drop shadows. The
// anti-slop rules in .opencode/skills/no-ai-slop/SKILL.md are applied here:
// no emoji icons, no pill overload, no uniform soft-shadow cards, no
// low-contrast grey body text, one deliberate accent.

use crate::tools as tools_mod;
use crate::{App, Collision, JobKind, JobState, Page, PickSlot, AUDIO_TIERS};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CursorIcon, FontFamily, FontId, Layout, Margin, Rect,
    Response, RichText, Rounding, Sense, Stroke, TextStyle,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Palette: one near-black ink, a clean paper white, a cool canvas, a single
// teal signal colour sampled from the product icon, and three tuned semantic
// colours.
// ---------------------------------------------------------------------------

const INK: Color32 = Color32::from_rgb(0x17, 0x1A, 0x1C);
const INK_SOFT: Color32 = Color32::from_rgb(0x3B, 0x41, 0x45);
const MUTED: Color32 = Color32::from_rgb(0x5F, 0x66, 0x6A);
const FAINT: Color32 = Color32::from_rgb(0x71, 0x77, 0x79);
const PAPER: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
const CANVAS: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xEE);
const HOVER_BG: Color32 = Color32::from_rgb(0xEC, 0xED, 0xE7);
const HAIRLINE: Color32 = Color32::from_rgb(0xDD, 0xDE, 0xD8);
const HAIRLINE_STRONG: Color32 = Color32::from_rgb(0xC7, 0xC9, 0xC2);

const RAIL: Color32 = Color32::from_rgb(0x19, 0x1C, 0x1E);
const RAIL_HOVER: Color32 = Color32::from_rgb(0x22, 0x26, 0x28);
const RAIL_ACTIVE: Color32 = Color32::from_rgb(0x28, 0x2C, 0x2E);
const RAIL_LINE: Color32 = Color32::from_rgb(0x30, 0x35, 0x37);
const RAIL_TEXT: Color32 = Color32::from_rgb(0xAA, 0xB0, 0xAC);
const RAIL_TEXT_HOVER: Color32 = Color32::from_rgb(0xDE, 0xE1, 0xDD);
const RAIL_TEXT_DIM: Color32 = Color32::from_rgb(0x86, 0x8D, 0x89);

const ACCENT: Color32 = Color32::from_rgb(0x0A, 0x6E, 0x96);
const ACCENT_DARK: Color32 = Color32::from_rgb(0x08, 0x5E, 0x80);
const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x06, 0x4A, 0x66);
const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE3, 0xF0, 0xF5);

const SUCCESS: Color32 = Color32::from_rgb(0x2E, 0x6B, 0x4F);
const SUCCESS_SOFT: Color32 = Color32::from_rgb(0xE2, 0xEF, 0xE7);
const WARN: Color32 = Color32::from_rgb(0x85, 0x60, 0x14);
const WARN_SOFT: Color32 = Color32::from_rgb(0xF4, 0xEC, 0xD8);
const ERROR: Color32 = Color32::from_rgb(0x9C, 0x2A, 0x1F);
const ERROR_SOFT: Color32 = Color32::from_rgb(0xF8, 0xE3, 0xDF);

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

fn display_family() -> FontFamily {
    FontFamily::Name("display".into())
}

fn display(size: f32) -> FontId {
    FontId::new(size, display_family())
}

fn sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

// ---------------------------------------------------------------------------
// Shared component language: one card, one button set, one chip, one tag,
// one hairline. Every page reuses these so nothing looks improvised.
// ---------------------------------------------------------------------------

pub struct ToolResult {
    pub label: String,
    pub ok: bool,
    pub note: String,
}

static TOOL_RESULT: Mutex<Option<ToolResult>> = Mutex::new(None);

fn style_workshop(ctx: &egui::Context) {
    let mut s = egui::Style {
        visuals: egui::Visuals::light(),
        ..Default::default()
    };
    let v = &mut s.visuals;

    v.window_fill = PAPER;
    v.panel_fill = CANVAS;
    v.extreme_bg_color = PAPER;
    v.faint_bg_color = HOVER_BG;
    v.hyperlink_color = ACCENT_DARK;
    v.striped = false;
    v.window_stroke = Stroke::new(1.0_f32, HAIRLINE);
    v.window_rounding = Rounding::same(8.0);
    v.menu_rounding = Rounding::same(6.0);
    v.window_shadow = egui::epaint::Shadow {
        offset: vec2(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: Color32::from_black_alpha(26),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: vec2(0.0, 3.0),
        blur: 10.0,
        spread: 0.0,
        color: Color32::from_black_alpha(20),
    };

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PAPER;
    w.noninteractive.weak_bg_fill = PAPER;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, HAIRLINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, INK);
    w.inactive.bg_fill = PAPER;
    w.inactive.weak_bg_fill = PAPER;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, HAIRLINE_STRONG);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, INK);
    w.hovered.bg_fill = PAPER;
    w.hovered.weak_bg_fill = Color32::from_rgb(0xF6, 0xF6, 0xF1);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, INK_SOFT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, INK);
    w.active.bg_fill = HOVER_BG;
    w.active.weak_bg_fill = HOVER_BG;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0_f32, INK);
    w.open.bg_fill = PAPER;
    w.open.weak_bg_fill = PAPER;
    w.open.bg_stroke = Stroke::new(1.0_f32, HAIRLINE_STRONG);
    w.open.fg_stroke = Stroke::new(1.0_f32, INK);

    for widget in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        widget.rounding = Rounding::same(5.0);
        widget.expansion = 0.0;
    }

    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    s.spacing.item_spacing = vec2(8.0, 8.0);
    s.spacing.button_padding = vec2(12.0, 6.0);
    s.spacing.interact_size = vec2(0.0, 30.0);
    s.spacing.window_margin = Margin::same(14.0);
    s.spacing.menu_margin = Margin::same(8.0);
    s.spacing.indent = 16.0;
    s.spacing.scroll.bar_width = 8.0;
    s.spacing.scroll.floating = false;
    s.animation_time = 0.12;

    s.text_styles = std::collections::BTreeMap::from([
        (TextStyle::Heading, display(26.0)),
        (TextStyle::Body, sans(14.0)),
        (TextStyle::Button, sans(13.0)),
        (TextStyle::Small, sans(11.5)),
        (TextStyle::Monospace, mono(12.0)),
    ]);

    ctx.set_style(s);
}

#[derive(Clone, Copy)]
enum BtnKind {
    Primary,
    Secondary,
    Ghost,
    DangerGhost,
}

fn button_w(
    ui: &mut egui::Ui,
    label: &str,
    kind: BtnKind,
    enabled: bool,
    width: Option<f32>,
) -> Response {
    let font = sans(13.0);
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE));
    let w = width.unwrap_or((galley.size().x + 28.0).max(64.0));
    let h = 34.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), sense);
    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();

    let (fill, stroke, text) = if !enabled {
        (
            Color32::from_rgb(0xEC, 0xEC, 0xE6),
            Stroke::new(1.0_f32, HAIRLINE),
            FAINT,
        )
    } else {
        match kind {
            BtnKind::Primary => (
                if pressed {
                    ACCENT_PRESSED
                } else if hovered {
                    ACCENT_DARK
                } else {
                    ACCENT
                },
                Stroke::NONE,
                Color32::WHITE,
            ),
            BtnKind::Secondary => (
                if pressed {
                    HOVER_BG
                } else if hovered {
                    Color32::from_rgb(0xF6, 0xF6, 0xF1)
                } else {
                    PAPER
                },
                Stroke::new(1.0_f32, if hovered { INK_SOFT } else { HAIRLINE_STRONG }),
                INK,
            ),
            BtnKind::Ghost => (
                if pressed {
                    HOVER_BG
                } else if hovered {
                    Color32::from_rgb(0xEF, 0xF0, 0xEA)
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
                if hovered { INK } else { INK_SOFT },
            ),
            BtnKind::DangerGhost => (
                if pressed {
                    Color32::from_rgb(0xF2, 0xD9, 0xD5)
                } else if hovered {
                    ERROR_SOFT
                } else {
                    Color32::TRANSPARENT
                },
                Stroke::NONE,
                ERROR,
            ),
        }
    };

    ui.painter().rect(rect, Rounding::same(5.0), fill, stroke);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

fn button(ui: &mut egui::Ui, label: &str, kind: BtnKind, enabled: bool) -> Response {
    button_w(ui, label, kind, enabled, None)
}

fn chip(ui: &mut egui::Ui, label: &str, selected: bool, enabled: bool) -> Response {
    let font = sans(12.5);
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE));
    let w = (galley.size().x + 18.0).max(44.0);
    let h = 30.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), sense);
    let hovered = enabled && resp.hovered();

    let (fill, stroke, text) = if selected {
        (
            if hovered { ACCENT_DARK } else { ACCENT },
            Stroke::new(1.0_f32, ACCENT_DARK),
            Color32::WHITE,
        )
    } else if !enabled {
        (
            Color32::from_rgb(0xEC, 0xEC, 0xE6),
            Stroke::new(1.0_f32, HAIRLINE),
            FAINT,
        )
    } else if hovered {
        (ACCENT_SOFT, Stroke::new(1.0_f32, ACCENT), ACCENT_DARK)
    } else {
        (PAPER, Stroke::new(1.0_f32, HAIRLINE_STRONG), INK_SOFT)
    };

    ui.painter().rect(rect, Rounding::same(4.0), fill, stroke);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

fn tag(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) -> Response {
    let font = sans(11.0);
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_owned(), font.clone(), fg));
    let w = galley.size().x + 16.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 21.0), Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(3.0), bg);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, text, font, fg);
    resp
}

fn rule(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0_f32, color));
}

fn hairline(ui: &mut egui::Ui) {
    rule(ui, HAIRLINE);
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(PAPER)
        .stroke(Stroke::new(1.0_f32, HAIRLINE))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            // Cards in the same column always share one width.
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).font(display(30.0)).color(INK));
    ui.add_space(4.0);
    ui.label(RichText::new(subtitle).font(sans(13.0)).color(MUTED));
    ui.add_space(16.0);
    hairline(ui);
    ui.add_space(20.0);
}

fn section_label(ui: &mut egui::Ui, title: &str, meta: Option<String>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(display(17.0)).color(INK));
        if let Some(meta) = meta {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(meta).font(sans(11.5)).color(MUTED));
            });
        }
    });
}

fn kv(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).font(sans(12.0)).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).font(sans(12.5)).color(INK));
        });
    });
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(sans(11.5)).color(MUTED));
}

fn tildify(path: &Path) -> String {
    let s = path.display().to_string();
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() && s.starts_with(&home) {
            return format!("~{}", &s[home.len()..]);
        }
    }
    s
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn parent_display(path: &Path) -> String {
    path.parent()
        .map(tildify)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "当前目录".into())
}

fn path_line(ui: &mut egui::Ui, label: &str, path: &Path) {
    ui.label(RichText::new(label).font(sans(11.5)).color(MUTED));
    let text = tildify(path);
    ui.add(egui::Label::new(RichText::new(&text).font(mono(10.5)).color(INK_SOFT)).truncate(true))
        .on_hover_text(text);
}

// ---------------------------------------------------------------------------
// Navigation rail
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum NavIcon {
    Video,
    Audio,
    Image,
    Tools,
    Queue,
    Settings,
}

fn paint_nav_icon(p: &egui::Painter, icon: NavIcon, r: Rect, c: Color32) {
    let s = Stroke::new(1.5_f32, c);
    match icon {
        NavIcon::Video => {
            p.rect_stroke(r.shrink2(vec2(1.0, 3.5)), Rounding::same(2.0), s);
            let cy = r.center().y;
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(r.center().x - 2.6, cy - 3.0),
                    pos2(r.center().x - 2.6, cy + 3.0),
                    pos2(r.center().x + 3.0, cy),
                ],
                c,
                Stroke::NONE,
            ));
        }
        NavIcon::Audio => {
            for (i, h) in [5.0_f32, 9.0, 13.0, 9.0, 5.0].iter().enumerate() {
                let x = r.left() + 2.0 + i as f32 * 3.0;
                p.line_segment(
                    [
                        pos2(x, r.center().y - h / 2.0),
                        pos2(x, r.center().y + h / 2.0),
                    ],
                    s,
                );
            }
        }
        NavIcon::Image => {
            p.rect_stroke(r.shrink2(vec2(1.0, 2.5)), Rounding::same(2.0), s);
            p.circle_filled(pos2(r.left() + 5.2, r.top() + 5.2), 1.5, c);
            p.add(egui::Shape::line(
                vec![
                    pos2(r.left() + 3.0, r.bottom() - 4.0),
                    pos2(r.center().x - 1.0, r.center().y + 1.0),
                    pos2(r.center().x + 1.5, r.center().y + 3.0),
                    pos2(r.right() - 3.0, r.top() + 6.0),
                ],
                s,
            ));
        }
        NavIcon::Tools => {
            p.line_segment(
                [
                    pos2(r.left() + 1.0, r.top() + 4.0),
                    pos2(r.left() + 6.0, r.center().y),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(r.left() + 1.0, r.bottom() - 4.0),
                    pos2(r.left() + 6.0, r.center().y),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(r.left() + 6.0, r.center().y),
                    pos2(r.right() - 3.0, r.center().y),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(r.right() - 5.5, r.center().y - 2.5),
                    pos2(r.right() - 2.5, r.center().y),
                ],
                s,
            );
            p.line_segment(
                [
                    pos2(r.right() - 5.5, r.center().y + 2.5),
                    pos2(r.right() - 2.5, r.center().y),
                ],
                s,
            );
        }
        NavIcon::Queue => {
            for i in 0..3 {
                let y = r.top() + 4.0 + i as f32 * 4.0;
                p.circle_filled(pos2(r.left() + 2.0, y), 1.2, c);
                p.line_segment([pos2(r.left() + 6.0, y), pos2(r.right() - 1.0, y)], s);
            }
        }
        NavIcon::Settings => {
            for (i, knob) in [5.0_f32, 11.0, 7.0].iter().enumerate() {
                let y = r.top() + 4.0 + i as f32 * 4.0;
                p.line_segment([pos2(r.left() + 1.0, y), pos2(r.right() - 1.0, y)], s);
                p.circle_filled(pos2(r.left() + knob, y), 1.8, c);
            }
        }
    }
}

fn nav_item(
    ui: &mut egui::Ui,
    icon: NavIcon,
    index: &str,
    label: &str,
    selected: bool,
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
    let hovered = resp.hovered();

    let bg = if selected {
        RAIL_ACTIVE
    } else if hovered {
        RAIL_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, Rounding::same(4.0), bg);
    }
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
            Rounding::same(0.0),
            ACCENT,
        );
    }

    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(16.0, 16.0));
    paint_nav_icon(
        ui.painter(),
        icon,
        icon_rect,
        if selected { Color32::WHITE } else { RAIL_TEXT },
    );

    let text_color = if selected {
        Color32::WHITE
    } else if hovered {
        RAIL_TEXT_HOVER
    } else {
        RAIL_TEXT
    };
    ui.painter().text(
        pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        sans(13.5),
        text_color,
    );
    ui.painter().text(
        pos2(rect.right() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        index,
        mono(10.0),
        RAIL_TEXT_DIM,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

fn rail_queue(ui: &mut egui::Ui, pending: usize) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 48.0), Sense::click());
    let hovered = resp.hovered();
    ui.painter().rect_filled(
        rect,
        Rounding::same(4.0),
        if hovered { RAIL_HOVER } else { RAIL_ACTIVE },
    );
    ui.painter()
        .circle_filled(pos2(rect.left() + 14.0, rect.center().y - 7.0), 3.0, ACCENT);
    ui.painter().text(
        pos2(rect.left() + 24.0, rect.center().y - 7.0),
        Align2::LEFT_CENTER,
        "转换进行中",
        sans(12.0),
        RAIL_TEXT_HOVER,
    );
    ui.painter().text(
        pos2(rect.left() + 14.0, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        format!("{pending} 个任务等待处理"),
        sans(11.0),
        RAIL_TEXT_DIM,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

fn side_panel(ctx: &egui::Context, app: &mut App) {
    egui::SidePanel::left("rail")
        .exact_width(200.0)
        .resizable(false)
        .frame(
            egui::Frame::none()
                .fill(RAIL)
                .inner_margin(Margin::symmetric(12.0, 18.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(icon) = &app.brand_icon {
                    ui.add(
                        egui::Image::from_texture(icon)
                            .fit_to_exact_size(vec2(32.0, 32.0))
                            .rounding(Rounding::same(8.0)),
                    );
                    ui.add_space(2.0);
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Ferry摇摆人")
                            .font(display(18.0))
                            .color(Color32::from_rgb(0xF4, 0xF2, 0xEC)),
                    );
                });
            });
            ui.add_space(16.0);
            rule(ui, RAIL_LINE);
            ui.add_space(10.0);

            for (page, icon, index, label) in [
                (Page::Video, NavIcon::Video, "01", "视频转换"),
                (Page::Audio, NavIcon::Audio, "02", "音频转换"),
                (Page::Image, NavIcon::Image, "03", "图片转换"),
                (Page::Tools, NavIcon::Tools, "04", "合并与混流"),
                (Page::Queue, NavIcon::Queue, "05", "任务列表"),
                (Page::Settings, NavIcon::Settings, "06", "设置"),
            ] {
                if nav_item(ui, icon, index, label, app.page == page).clicked() {
                    app.page = page;
                }
            }

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.label(
                    RichText::new("v0.1.0 · 独立重建")
                        .font(mono(9.5))
                        .color(RAIL_TEXT_DIM),
                );
                ui.add_space(10.0);
                let ff = match &app.ffmpeg_found {
                    Some(v) if !v.contains("未找到") => ("ffmpeg 已就绪", SUCCESS),
                    Some(_) => ("未找到 ffmpeg", WARN),
                    None => ("正在检查 ffmpeg", RAIL_TEXT_DIM),
                };
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 3.0, ff.1);
                    ui.label(RichText::new(ff.0).font(sans(10.5)).color(RAIL_TEXT_DIM));
                });
                ui.add_space(6.0);
                let pending = app
                    .jobs
                    .iter()
                    .filter(|j| matches!(j.state, JobState::Pending | JobState::Running))
                    .count();
                if pending > 0 && rail_queue(ui, pending).clicked() {
                    app.page = Page::Queue;
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn status_bar(ctx: &egui::Context, app: &App) {
    egui::TopBottomPanel::bottom("statusbar")
        .frame(
            egui::Frame::none()
                .fill(PAPER)
                .inner_margin(Margin::symmetric(20.0, 8.0)),
        )
        .show(ctx, |ui| {
            let top = ui.max_rect().top();
            ui.painter()
                .hline(ui.max_rect().x_range(), top, Stroke::new(1.0_f32, HAIRLINE));
            ui.horizontal(|ui| {
                let (ok, text) = match &app.ffmpeg_found {
                    Some(v) if !v.contains("未找到") => (true, format!("ffmpeg: {v}")),
                    Some(v) => (false, format!("ffmpeg: {v}")),
                    None => (false, "ffmpeg: 检测中…".into()),
                };
                let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 3.0, if ok { SUCCESS } else { WARN });
                ui.add(
                    egui::Label::new(RichText::new(text).font(sans(11.0)).color(MUTED))
                        .truncate(true),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !app.status.is_empty() {
                        let color = if app.status.starts_with('✗') {
                            ERROR
                        } else {
                            INK_SOFT
                        };
                        ui.add(
                            egui::Label::new(
                                RichText::new(&app.status).font(sans(11.0)).color(color),
                            )
                            .truncate(true),
                        );
                    }
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

pub fn draw(ctx: &egui::Context, app: &mut App) {
    style_workshop(ctx);
    side_panel(ctx, app);
    status_bar(ctx, app);

    handle_drops(ctx, app);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(CANVAS)
                .inner_margin(Margin::symmetric(26.0, 22.0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Keep the scroll bar in the light system even when the
                    // surrounding theme is swapped at runtime.
                    let v = ui.visuals_mut();
                    v.extreme_bg_color = CANVAS;
                    v.widgets.inactive.bg_fill = HAIRLINE_STRONG;
                    v.widgets.hovered.bg_fill = INK_SOFT;
                    v.widgets.active.bg_fill = ACCENT;
                    match app.page {
                        Page::Video => convert_page(ctx, ui, app, TargetKind::Video),
                        Page::Audio => convert_page(ctx, ui, app, TargetKind::Audio),
                        Page::Image => convert_page(ctx, ui, app, TargetKind::Image),
                        Page::Tools => tools_page(ui, app),
                        Page::Queue => queue_page(ui, app),
                        Page::Settings => settings_page(ui, app),
                    }
                });
        });

    drop_overlay(ctx, app);
}

fn handle_drops(ctx: &egui::Context, app: &mut App) {
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .filter(|p| p.is_file())
            .collect()
    });
    if dropped.is_empty() {
        return;
    }
    let n = dropped.len();
    match app.page {
        Page::Video => app.video_files.extend(dropped),
        Page::Audio => app.audio_files.extend(dropped),
        Page::Image => app.image_files.extend(dropped),
        Page::Tools => app.join_files.extend(dropped),
        Page::Queue | Page::Settings => return,
    }
    app.status = format!("已添加 {n} 个文件");
}

fn drop_overlay(ctx: &egui::Context, app: &App) {
    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    if !hovering {
        return;
    }
    let target = match app.page {
        Page::Video => Some("视频列表"),
        Page::Audio => Some("音频列表"),
        Page::Image => Some("图片列表"),
        Page::Tools => Some("合并列表"),
        Page::Queue | Page::Settings => None,
    };
    let Some(target) = target else { return };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop-overlay"),
    ));
    let rect = ctx.screen_rect();
    painter.rect_filled(
        rect,
        Rounding::same(0.0),
        Color32::from_rgba_premultiplied(0x17, 0x1A, 0x1C, 190),
    );
    painter.rect_stroke(
        rect.shrink(18.0),
        Rounding::same(8.0),
        Stroke::new(1.5_f32, ACCENT),
    );
    painter.text(
        rect.center() - vec2(0.0, 14.0),
        Align2::CENTER_CENTER,
        "松开即可添加文件",
        display(30.0),
        Color32::WHITE,
    );
    painter.text(
        rect.center() + vec2(0.0, 20.0),
        Align2::CENTER_CENTER,
        format!("添加到{target}"),
        sans(13.0),
        RAIL_TEXT,
    );
    ctx.request_repaint();
}

// ---------------------------------------------------------------------------
// Convert pages
// ---------------------------------------------------------------------------

const VIDEO_FORMATS: [&str; 7] = ["mp4", "mkv", "webm", "gif", "wmv", "avi", "mov"];
const AUDIO_FORMATS: [&str; 6] = ["mp3", "wav", "aac", "flac", "ogg", "m4a"];
const IMAGE_FORMATS: [&str; 8] = ["jpg", "png", "webp", "bmp", "gif", "tiff", "ico", "tga"];

#[derive(Clone, Copy, PartialEq)]
pub enum TargetKind {
    Video,
    Audio,
    Image,
}

impl TargetKind {
    fn formats(self) -> &'static [&'static str] {
        match self {
            TargetKind::Video => &VIDEO_FORMATS,
            TargetKind::Audio => &AUDIO_FORMATS,
            TargetKind::Image => &IMAGE_FORMATS,
        }
    }

    fn files_mut(self, app: &mut App) -> &mut Vec<PathBuf> {
        match self {
            TargetKind::Video => &mut app.video_files,
            TargetKind::Audio => &mut app.audio_files,
            TargetKind::Image => &mut app.image_files,
        }
    }

    fn files(self, app: &App) -> &Vec<PathBuf> {
        match self {
            TargetKind::Video => &app.video_files,
            TargetKind::Audio => &app.audio_files,
            TargetKind::Image => &app.image_files,
        }
    }

    fn target_mut(self, app: &mut App) -> &mut String {
        match self {
            TargetKind::Video => &mut app.video_target,
            TargetKind::Audio => &mut app.audio_target,
            TargetKind::Image => &mut app.image_target,
        }
    }

    fn title(self) -> &'static str {
        match self {
            TargetKind::Video => "视频",
            TargetKind::Audio => "音频",
            TargetKind::Image => "图片",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            TargetKind::Video => "把视频换成另一种格式。转换由 ffmpeg 完成，原文件保留。",
            TargetKind::Audio => "把音频换成另一种格式。转换由 ffmpeg 完成，原文件保留。",
            TargetKind::Image => "把图片换成另一种格式。使用内置解码器，不需要 ffmpeg。",
        }
    }

    fn encoding(self, target: &str, audio_quality: usize) -> String {
        let kbps = AUDIO_TIERS.get(audio_quality).map(|t| t.1).unwrap_or(192);
        match self {
            TargetKind::Video => match target {
                "webm" => "VP9 · Opus 96k".into(),
                "wmv" => "WMV2 · WMA 128k".into(),
                "gif" => "15 fps · 480px · 16 色".into(),
                _ => "H.264 · AAC 128k".into(),
            },
            TargetKind::Audio => match target {
                "mp3" => format!("MP3 · {kbps} kbps"),
                "aac" | "m4a" => format!("AAC · {kbps} kbps"),
                "wav" => "PCM 16-bit".into(),
                "flac" => "FLAC 无损".into(),
                "ogg" => "Vorbis q5".into(),
                _ => "ffmpeg 默认".into(),
            },
            TargetKind::Image => match target {
                "jpg" | "webp" => "质量 90 · 内置解码".into(),
                "png" | "bmp" | "tga" => "无损 · 内置解码".into(),
                _ => "内置解码".into(),
            },
        }
    }

    fn filter(self) -> Option<String> {
        // Do NOT pass a type filter to zenity: filter syntax varies between
        // zenity versions/builds and has left files unselectable (greyed out)
        // for users. The app validates formats when converting instead, and an
        // "All files" dialog lets every file be chosen and confirmed.
        let _ = self;
        None
    }
}

fn convert_page(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut App, kind: TargetKind) {
    page_header(ui, &format!("{}转换", kind.title()), kind.hint());

    let full = ui.available_width();
    let aside_w = 288.0;
    let gap = 20.0;

    if full >= 720.0 {
        ui.horizontal_top(|ui| {
            let main_w = (full - aside_w - gap).max(280.0);
            ui.allocate_ui_with_layout(vec2(main_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(main_w);
                convert_main(ui, app, kind);
            });
            ui.add_space(gap);
            ui.allocate_ui_with_layout(vec2(aside_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(aside_w);
                convert_aside(ui, app, kind);
            });
        });
    } else {
        convert_main(ui, app, kind);
        ui.add_space(22.0);
        convert_aside(ui, app, kind);
    }

    if app.pick_slot == PickSlot::None {
        if let Some(Ok(paths)) = tools_mod::take_pick_result() {
            kind.files_mut(app).extend(paths);
        }
    }

    if tools_mod::picker_busy() {
        ctx.request_repaint();
    }
}

fn convert_main(ui: &mut egui::Ui, app: &mut App, kind: TargetKind) {
    section_label(ui, "输出格式", None);
    ui.add_space(9.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(7.0, 7.0);
        for f in kind.formats() {
            let sel = kind.target_mut(app) == f;
            if chip(ui, &f.to_uppercase(), sel, true).clicked() {
                *kind.target_mut(app) = f.to_string();
            }
        }
    });

    ui.add_space(24.0);
    let count = kind.files(app).len();
    section_label(
        ui,
        "待转换文件",
        Some(if count == 0 {
            "未选择".into()
        } else {
            format!("{count} 个")
        }),
    );
    ui.add_space(9.0);

    card(ui, |ui| {
        let files = kind.files_mut(app).clone();
        if files.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new("还没有文件").font(display(19.0)).color(INK));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "拖入文件，或从文件夹选择。支持 {}。",
                    kind.formats().join(" / ").to_uppercase()
                ))
                .font(sans(12.0))
                .color(MUTED),
            );
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let busy = tools_mod::picker_busy();
                if button(ui, "添加文件…", BtnKind::Primary, !busy).clicked() {
                    app.pick_slot = PickSlot::None;
                    tools_mod::open_file_picker(format!("选择{}文件", kind.title()), kind.filter());
                }
                if button(ui, "从剪贴板粘贴", BtnKind::Ghost, true).clicked() {
                    paste_paths(app, kind);
                }
            });
            ui.add_space(4.0);
        } else {
            for (i, f) in files.iter().enumerate() {
                let removed = file_row(ui, i, file_name(f), parent_display(f), &tildify(f));
                if removed {
                    kind.files_mut(app).remove(i);
                }
                if i + 1 < files.len() {
                    hairline(ui);
                }
            }

            ui.add_space(4.0);
            hairline(ui);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let busy = tools_mod::picker_busy();
                if button(ui, "添加文件…", BtnKind::Secondary, !busy).clicked() {
                    app.pick_slot = PickSlot::None;
                    tools_mod::open_file_picker(format!("选择{}文件", kind.title()), kind.filter());
                }
                if button(ui, "从剪贴板粘贴", BtnKind::Ghost, true).clicked() {
                    paste_paths(app, kind);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "清空列表", BtnKind::DangerGhost, true).clicked() {
                        kind.files_mut(app).clear();
                    }
                });
            });
        }
    });
}

fn file_row(ui: &mut egui::Ui, index: usize, name: String, dir: String, full_path: &str) -> bool {
    let w = ui.available_width();
    let mut removed = false;
    ui.horizontal(|ui| {
        ui.set_width(w);
        ui.label(
            RichText::new(format!("{:02}", index + 1))
                .font(mono(10.5))
                .color(FAINT),
        );
        let text_w = (w - 76.0).max(90.0);
        ui.allocate_ui_with_layout(vec2(text_w, 36.0), Layout::top_down(Align::Min), |ui| {
            ui.add(
                egui::Label::new(RichText::new(&name).font(sans(13.5)).color(INK)).truncate(true),
            )
            .on_hover_text(full_path);
            ui.add(
                egui::Label::new(RichText::new(&dir).font(mono(10.0)).color(MUTED)).truncate(true),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, "移除", BtnKind::Ghost, true).clicked() {
                removed = true;
            }
        });
    });
    ui.add_space(4.0);
    removed
}

fn convert_aside(ui: &mut egui::Ui, app: &mut App, kind: TargetKind) {
    let target = kind.target_mut(app).clone();
    let files = kind.files(app).clone();
    let count = files.len();

    card(ui, |ui| {
        ui.label(RichText::new("任务摘要").font(display(18.0)).color(INK));
        ui.add_space(12.0);

        kv(ui, "输出格式", &target.to_uppercase());
        kv(
            ui,
            "编码",
            &kind.encoding(&target, app.settings.audio_quality),
        );

        if kind == TargetKind::Image {
            ui.add_space(10.0);
            ui.label(RichText::new("输出尺寸").font(sans(11.5)).color(MUTED));
            ui.add_space(6.0);
            if target == "ico" {
                caption(ui, "ICO 固定输出单帧 256 px。");
            } else {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    ui.label(RichText::new("宽 px").font(sans(11.0)).color(MUTED));
                    ui.add_sized(
                        [62.0, 28.0],
                        egui::DragValue::new(&mut app.settings.image_width)
                            .clamp_range(0..=crate::imaging::MAX_OUTPUT_SIDE)
                            .speed(1.0),
                    );
                    ui.label(RichText::new("高 px").font(sans(11.0)).color(MUTED));
                    ui.add_sized(
                        [62.0, 28.0],
                        egui::DragValue::new(&mut app.settings.image_height)
                            .clamp_range(0..=crate::imaging::MAX_OUTPUT_SIDE)
                            .speed(1.0),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let has_size = app.settings.image_size().is_some();
                    if button(ui, "原始尺寸", BtnKind::Ghost, has_size).clicked() {
                        app.settings.image_width = 0;
                        app.settings.image_height = 0;
                    }
                    caption(ui, "0 为原始；单边等比缩放");
                });
            }
        }

        if kind == TargetKind::Audio && matches!(target.as_str(), "mp3" | "aac" | "m4a") {
            ui.add_space(10.0);
            ui.label(RichText::new("音质").font(sans(11.5)).color(MUTED));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for (i, (name, kbps)) in AUDIO_TIERS.iter().enumerate() {
                    let label = format!("{name} {kbps}");
                    if chip(ui, &label, app.settings.audio_quality == i, true).clicked() {
                        app.settings.audio_quality = i;
                    }
                }
            });
        }

        ui.add_space(12.0);
        hairline(ui);
        ui.add_space(12.0);

        match &app.settings.output_dir {
            Some(dir) => path_line(ui, "输出位置", dir),
            None => {
                ui.label(RichText::new("输出位置").font(sans(11.5)).color(MUTED));
                ui.label(
                    RichText::new("与源文件相同")
                        .font(sans(12.0))
                        .color(INK_SOFT),
                );
            }
        }

        ui.add_space(14.0);
        let label = if count == 0 {
            "先添加文件".to_string()
        } else {
            format!("转换 {count} 个文件为 {}", target.to_uppercase())
        };
        let width = ui.available_width();
        if button_w(ui, &label, BtnKind::Primary, count > 0, Some(width)).clicked() {
            start_convert(app, kind, files.clone(), target.clone());
        }
        ui.add_space(6.0);
        caption(
            ui,
            "转换在后台逐个进行，原文件不会被修改。同名文件的处理方式可在设置里调整。",
        );
    });

    ui.add_space(12.0);

    let pending = app
        .jobs
        .iter()
        .filter(|j| matches!(j.state, JobState::Pending | JobState::Running))
        .count();
    let done = app
        .jobs
        .iter()
        .filter(|j| matches!(j.state, JobState::Done))
        .count();

    if !app.jobs.is_empty() {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("队列").font(display(16.0)).color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if pending > 0 {
                        tag(ui, &format!("{pending} 进行中"), ACCENT_DARK, ACCENT_SOFT);
                    } else {
                        tag(ui, "空闲", MUTED, HOVER_BG);
                    }
                });
            });
            ui.add_space(6.0);
            caption(
                ui,
                &format!("已完成 {done} 个；队列共 {} 个任务。", app.jobs.len()),
            );
            ui.add_space(8.0);
            let width = ui.available_width();
            if button_w(ui, "查看任务列表", BtnKind::Secondary, true, Some(width)).clicked() {
                app.page = Page::Queue;
            }
        });
    }
}

fn paste_paths(app: &mut App, kind: TargetKind) {
    // clipboard paste without an extra dependency: run xclip/zenity-free read
    let text = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut added = 0;
    for line in text.lines() {
        let p = PathBuf::from(line.trim().trim_matches('"'));
        if p.exists() {
            kind.files_mut(app).push(p);
            added += 1;
        }
    }
    app.status = if added > 0 {
        format!("已添加 {added} 个文件")
    } else {
        "剪贴板里没有有效文件路径".into()
    };
}

fn start_convert(app: &mut App, kind: TargetKind, files: Vec<PathBuf>, target: String) {
    let job_kind = match kind {
        TargetKind::Image => JobKind::Image,
        _ => JobKind::Convert,
    };
    app.add_jobs(&files, &target, job_kind);
    // The image size is a per-conversion parameter, so remember the value
    // that was actually used even if the settings page was never opened.
    app.settings.save();
    app.page = Page::Queue;
}

// ---------------------------------------------------------------------------
// Tools page
// ---------------------------------------------------------------------------

fn tools_page(ui: &mut egui::Ui, app: &mut App) {
    page_header(
        ui,
        "合并与混流",
        "合并把多个同类文件接成一个；混流把视频画面和音轨装进同一个文件。两者都交给 ffmpeg。",
    );

    let full = ui.available_width();
    if full >= 760.0 {
        let gap = 20.0;
        let left_w = (full * 0.56).max(360.0);
        let right_w = full - left_w - gap;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(left_w);
                join_card(ui, app);
            });
            ui.add_space(gap);
            ui.allocate_ui_with_layout(vec2(right_w, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(right_w);
                mux_card(ui, app);
            });
        });
    } else {
        join_card(ui, app);
        ui.add_space(18.0);
        mux_card(ui, app);
    }

    // Collect picker results for the tools page. The button that opened the
    // dialog records its slot so changing an already filled mux input works.
    if app.pick_slot != PickSlot::None {
        if let Some(res) = tools_mod::take_pick_result() {
            if let Ok(paths) = res {
                match app.pick_slot {
                    PickSlot::Join => app.join_files.extend(paths),
                    PickSlot::MuxVideo => {
                        if let Some(p) = paths.first() {
                            app.mux_video = Some(p.clone());
                        }
                    }
                    PickSlot::MuxAudio => {
                        if let Some(p) = paths.first() {
                            app.mux_audio = Some(p.clone());
                        }
                    }
                    PickSlot::None => {}
                }
            }
            app.pick_slot = PickSlot::None;
        }
    }
}

fn join_card(ui: &mut egui::Ui, app: &mut App) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("合并").font(display(19.0)).color(INK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let n = app.join_files.len();
                if n > 0 {
                    tag(ui, &format!("{n} 个文件"), INK_SOFT, HOVER_BG);
                }
            });
        });
        ui.add_space(4.0);
        caption(
            ui,
            "按列表顺序拼接。快速拼接直接复用原编码，要求各文件编码一致。",
        );
        ui.add_space(12.0);

        let files = app.join_files.clone();
        if files.is_empty() {
            ui.label(
                RichText::new("还没有要合并的文件")
                    .font(sans(13.0))
                    .color(MUTED),
            );
            ui.add_space(10.0);
        } else {
            for (i, f) in files.iter().enumerate() {
                let removed = file_row(ui, i, file_name(f), parent_display(f), &tildify(f));
                if removed {
                    app.join_files.remove(i);
                }
                if i + 1 < files.len() {
                    hairline(ui);
                }
            }
            ui.add_space(4.0);
            hairline(ui);
            ui.add_space(8.0);
        }

        ui.horizontal(|ui| {
            let busy = tools_mod::picker_busy();
            let add_kind = if files.is_empty() {
                BtnKind::Primary
            } else {
                BtnKind::Secondary
            };
            if button(ui, "添加文件…", add_kind, !busy).clicked() {
                app.pick_slot = PickSlot::Join;
                tools_mod::open_file_picker(
                    "选择要合并的文件".into(),
                    Some("媒体文件|*.mp4;*.mkv;*.ts;*.mp3;*.aac;*.wav;*.flac;*.ogg".into()),
                );
            }
            if !files.is_empty() && button(ui, "清空", BtnKind::DangerGhost, true).clicked() {
                app.join_files.clear();
            }
        });

        ui.add_space(14.0);
        ui.label(RichText::new("拼接方式").font(sans(11.5)).color(MUTED));
        ui.add_space(6.0);
        let copy = app.join_mode == crate::JoinMode::JoinCopy;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(7.0, 7.0);
            if chip(ui, "快速拼接 · 不重编码", copy, true).clicked() {
                app.join_mode = crate::JoinMode::JoinCopy;
            }
            if chip(ui, "重新编码 · H.264 + AAC", !copy, true).clicked() {
                app.join_mode = crate::JoinMode::Join;
            }
        });
        ui.add_space(8.0);
        caption(
            ui,
            if app.join_mode == crate::JoinMode::JoinCopy {
                "直接复用原编码，速度快；各文件编码不一致时可能失败。"
            } else {
                "统一重新编码为 H.264 + AAC，兼容性更好，速度较慢。"
            },
        );

        if app.join_files.len() >= 2 {
            let out_dir = app
                .settings
                .output_dir
                .clone()
                .or_else(|| app.join_files[0].parent().map(|p| p.to_path_buf()))
                .unwrap_or_default();
            let out = out_dir.join(format!("merged_{}.mp4", timestamp()));
            ui.add_space(12.0);
            path_line(ui, "输出文件", &out);
            ui.add_space(10.0);
            let label = format!("合并 {} 个文件", app.join_files.len());
            if button(ui, &label, BtnKind::Primary, true).clicked() {
                let files = app.join_files.clone();
                std::thread::spawn(move || {
                    let r = crate::engine::run_join(&files, &out, copy);
                    report_tool("合并".into(), r);
                });
                app.status = "合并已开始".into();
            }
        }
    });
}

fn mux_card(ui: &mut egui::Ui, app: &mut App) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("混流").font(display(19.0)).color(INK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                tag(ui, "视频 + 音轨", INK_SOFT, HOVER_BG);
            });
        });
        ui.add_space(4.0);
        caption(ui, "把视频画面和另一个音频文件装进同一个容器，不重新编码。");
        ui.add_space(14.0);

        let video_primary = if app.mux_video.is_none() {
            BtnKind::Primary
        } else {
            BtnKind::Secondary
        };
        mux_slot(
            ui,
            "视频",
            app.mux_video.as_ref(),
            "未选择视频",
            video_primary,
            || {
                app.pick_slot = PickSlot::MuxVideo;
                tools_mod::open_file_picker(
                    "选择视频".into(),
                    Some("视频文件|*.mp4;*.mkv;*.webm;*.avi;*.mov".into()),
                );
            },
        );
        ui.add_space(8.0);
        hairline(ui);
        ui.add_space(8.0);
        let audio_primary = if app.mux_video.is_some() && app.mux_audio.is_none() {
            BtnKind::Primary
        } else {
            BtnKind::Secondary
        };
        mux_slot(
            ui,
            "音频",
            app.mux_audio.as_ref(),
            "未选择音频",
            audio_primary,
            || {
                app.pick_slot = PickSlot::MuxAudio;
                tools_mod::open_file_picker(
                    "选择音频".into(),
                    Some("音频文件|*.mp3;*.aac;*.m4a;*.wav;*.flac;*.ogg".into()),
                );
            },
        );

        let ready = app.mux_video.is_some() && app.mux_audio.is_some();
        if ready {
            let v = app.mux_video.clone().unwrap();
            let a = app.mux_audio.clone().unwrap();
            let out_dir = v.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let out = out_dir.join(format!("muxed_{}.mp4", timestamp()));
            ui.add_space(12.0);
            path_line(ui, "输出文件", &out);
            ui.add_space(10.0);
            if button(ui, "混流为一个视频", BtnKind::Primary, true).clicked() {
                std::thread::spawn(move || {
                    let r = crate::engine::run_mux(&v, &a, &out);
                    report_tool("混流".into(), r);
                });
                app.status = "混流已开始".into();
            }
        } else {
            ui.add_space(12.0);
            let width = ui.available_width();
            button_w(ui, "混流为一个视频", BtnKind::Primary, false, Some(width));
            ui.add_space(6.0);
            caption(ui, "选择视频和音频后即可开始。");
        }
    });
}

fn mux_slot(
    ui: &mut egui::Ui,
    label: &str,
    path: Option<&PathBuf>,
    empty: &str,
    select_kind: BtnKind,
    on_pick: impl FnOnce(),
) {
    ui.label(RichText::new(label).font(sans(11.5)).color(MUTED));
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        if let Some(p) = path {
            ui.add(
                egui::Label::new(RichText::new(file_name(p)).font(sans(12.5)).color(INK))
                    .truncate(true),
            )
            .on_hover_text(tildify(p));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "更换…", BtnKind::Ghost, true).clicked() {
                    on_pick();
                }
            });
        } else {
            ui.label(RichText::new(empty).font(sans(12.5)).color(FAINT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "选择…", select_kind, true).clicked() {
                    on_pick();
                }
            });
        }
    });
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Queue page
// ---------------------------------------------------------------------------

fn resize_label((w, h): (u32, u32)) -> String {
    match (w, h) {
        (w, 0) => format!("输出宽度 {w} px（等比）"),
        (0, h) => format!("输出高度 {h} px（等比）"),
        (w, h) => format!("输出尺寸 {w}×{h}"),
    }
}

fn queue_page(ui: &mut egui::Ui, app: &mut App) {
    let total = app.jobs.len();
    let pending = app
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Pending)
        .count();
    let running = app
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Running)
        .count();
    let done = app
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Done)
        .count();
    let failed = app
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Failed)
        .count();
    let skipped = app
        .jobs
        .iter()
        .filter(|j| j.state == JobState::Skipped)
        .count();

    page_header(
        ui,
        "任务列表",
        "转换在后台逐个进行，不会改动原文件。失败的任务可以重试。",
    );

    if total > 0 {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("共 {total} 个任务"))
                    .font(display(16.0))
                    .color(INK),
            );
            ui.add_space(8.0);
            count_dot(ui, pending, "等待", MUTED);
            count_dot(ui, running, "进行中", ACCENT);
            count_dot(ui, done, "完成", SUCCESS);
            count_dot(ui, failed, "失败", ERROR);
            if skipped > 0 {
                count_dot(ui, skipped, "跳过", WARN);
            }
        });
        ui.add_space(12.0);
    }

    ui.horizontal(|ui| {
        if button(
            ui,
            "清除已结束",
            BtnKind::Secondary,
            done + failed + skipped > 0,
        )
        .clicked()
        {
            app.drop_finished();
        }
        if button(ui, "重试失败", BtnKind::Secondary, failed + skipped > 0).clicked() {
            for j in app.jobs.iter_mut() {
                if matches!(j.state, JobState::Failed | JobState::Skipped) {
                    j.state = JobState::Pending;
                    j.note.clear();
                }
            }
        }
    });

    ui.add_space(14.0);

    if app.jobs.is_empty() {
        let empty_w = ui.available_width().min(620.0);
        ui.allocate_ui_with_layout(vec2(empty_w, 0.0), Layout::top_down(Align::Min), |ui| {
            card(ui, |ui| {
                ui.label(RichText::new("队列是空的").font(display(19.0)).color(INK));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("在视频、音频或图片页添加文件后，任务会在这里排队。")
                        .font(sans(12.0))
                        .color(MUTED),
                );
                ui.add_space(14.0);
                if button(ui, "去添加文件", BtnKind::Primary, true).clicked() {
                    app.page = Page::Video;
                }
                ui.add_space(2.0);
            });
        });
        return;
    }

    card(ui, |ui| {
        for (idx, j) in app.jobs.iter_mut().enumerate() {
            if idx > 0 {
                hairline(ui);
            }
            ui.add_space(9.0);
            ui.horizontal(|ui| {
                let (label, fg, bg) = match j.state {
                    JobState::Pending => ("等待中", MUTED, HOVER_BG),
                    JobState::Running => ("转换中", ACCENT_DARK, ACCENT_SOFT),
                    JobState::Done => ("完成", SUCCESS, SUCCESS_SOFT),
                    JobState::Failed => ("失败", ERROR, ERROR_SOFT),
                    JobState::Skipped => ("已跳过", WARN, WARN_SOFT),
                };
                tag(ui, label, fg, bg);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(file_name(&j.source))
                                .font(sans(13.0))
                                .color(INK),
                        )
                        .truncate(true),
                    )
                    .on_hover_text(tildify(&j.source));
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("→ {}", tildify(&j.target_path)))
                                .font(mono(10.0))
                                .color(MUTED),
                        )
                        .truncate(true),
                    );
                    if matches!(j.state, JobState::Pending | JobState::Running) {
                        if let Some(size) = j.resize {
                            ui.label(
                                RichText::new(resize_label(size))
                                    .font(sans(10.0))
                                    .color(MUTED),
                            );
                        }
                    }
                    if !j.note.is_empty() {
                        let color = if j.state == JobState::Failed {
                            ERROR
                        } else {
                            MUTED
                        };
                        ui.label(RichText::new(&j.note).font(sans(10.5)).color(color));
                    }
                });
            });
            ui.add_space(9.0);
        }
    });
}

fn count_dot(ui: &mut egui::Ui, count: usize, label: &str, color: Color32) {
    if count == 0 {
        return;
    }
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.0, color);
        ui.label(
            RichText::new(format!("{label} {count}"))
                .font(sans(11.0))
                .color(MUTED),
        );
    });
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

fn settings_page(ui: &mut egui::Ui, app: &mut App) {
    let cfg = tildify(&crate::dirs_config_dir().join("ferry.json"));
    let subtitle = format!("输出位置、同名文件处理和默认音质。设置保存在 {cfg}。");

    ui.add_space(2.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("设置").font(display(30.0)).color(INK));
            ui.add_space(4.0);
            ui.label(RichText::new(subtitle).font(sans(13.0)).color(MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if button(ui, "保存设置", BtnKind::Primary, true).clicked() {
                app.settings.save();
                app.status = "设置已保存".into();
            }
        });
    });
    ui.add_space(16.0);
    hairline(ui);
    ui.add_space(20.0);

    ui.scope(|ui| {
        ui.set_max_width(760.0);

        card(ui, |ui| {
            section_label(ui, "输出", None);
            ui.add_space(12.0);

            let full = ui.available_width();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((full - 200.0).max(140.0));
                    ui.label(RichText::new("输出目录").font(sans(11.5)).color(MUTED));
                    ui.add_space(2.0);
                    match &app.settings.output_dir {
                        Some(d) => {
                            let text = tildify(d);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&text)
                                        .font(mono(10.5))
                                        .color(INK_SOFT),
                                )
                                .truncate(true),
                            )
                            .on_hover_text(text);
                        }
                        None => {
                            ui.label(
                                RichText::new("与源文件相同")
                                    .font(sans(12.0))
                                    .color(INK_SOFT),
                            );
                        }
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "选择…", BtnKind::Secondary, true).clicked() {
                        app.pick_slot = PickSlot::None;
                        tools_mod::open_dir_picker("选择输出目录".into());
                    }
                    if app.settings.output_dir.is_some()
                        && button(ui, "恢复默认", BtnKind::Ghost, true).clicked()
                    {
                        app.settings.output_dir = None;
                    }
                });
            });

            ui.add_space(12.0);
            hairline(ui);
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("文件重名时").font(sans(12.0)).color(MUTED));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(7.0, 7.0);
                    if chip(
                        ui,
                        "覆盖",
                        app.settings.collision == Collision::Overwrite,
                        true,
                    )
                    .clicked()
                    {
                        app.settings.collision = Collision::Overwrite;
                    }
                    if chip(
                        ui,
                        "自动改名",
                        app.settings.collision == Collision::Rename,
                        true,
                    )
                    .clicked()
                    {
                        app.settings.collision = Collision::Rename;
                    }
                    if chip(
                        ui,
                        "跳过",
                        app.settings.collision == Collision::Skip,
                        true,
                    )
                    .clicked()
                    {
                        app.settings.collision = Collision::Skip;
                    }
                });
            });
            ui.add_space(6.0);
            caption(
                ui,
                match app.settings.collision {
                    Collision::Overwrite => "同名文件会被直接覆盖。",
                    Collision::Rename => "同名文件会自动加序号，不覆盖现有文件。",
                    Collision::Skip => "同名文件会跳过，保留现有文件。",
                },
            );
        });

        ui.add_space(14.0);

        let audio_card = |ui: &mut egui::Ui, app: &mut App| {
            card(ui, |ui| {
                section_label(ui, "音质", None);
                ui.add_space(4.0);
                caption(ui, "用于 MP3、AAC 和 M4A 输出。");
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(7.0, 7.0);
                    for (i, (name, kbps)) in AUDIO_TIERS.iter().enumerate() {
                        let label = format!("{name} {kbps}k");
                        if chip(ui, &label, app.settings.audio_quality == i, true).clicked() {
                            app.settings.audio_quality = i;
                        }
                    }
                });
            });
        };
        let about_card = |ui: &mut egui::Ui| {
            card(ui, |ui| {
                section_label(ui, "关于", None);
                ui.add_space(10.0);
                kv(ui, "版本", "0.1.0");
                ui.add_space(10.0);
                caption(
                    ui,
                    "独立重建版，不包含原始安装包中的代码。视频、音频、合并与混流需要 ffmpeg；图片转换使用内置解码。",
                );
            });
        };

        let full = ui.available_width();
        if full >= 700.0 {
            let gap = 14.0;
            let left_w = (full * 0.55).max(340.0);
            let right_w = full - left_w - gap;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(left_w, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(left_w);
                        audio_card(ui, app);
                    },
                );
                ui.add_space(gap);
                ui.allocate_ui_with_layout(
                    vec2(right_w, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(right_w);
                        about_card(ui);
                    },
                );
            });
        } else {
            audio_card(ui, app);
            ui.add_space(14.0);
            about_card(ui);
        }

    });

    if app.pick_slot == PickSlot::None {
        if let Some(Ok(paths)) = tools_mod::take_pick_result() {
            if let Some(p) = paths.first() {
                if p.is_dir() {
                    app.settings.output_dir = Some(p.clone());
                }
            }
        }
    }
}

pub fn report_tool(label: String, r: Result<(), String>) {
    let (ok, note) = match r {
        Ok(_) => (true, "完成".into()),
        Err(e) => (false, e),
    };
    if let Ok(mut g) = TOOL_RESULT.lock() {
        *g = Some(ToolResult { label, ok, note });
    }
}

pub fn take_tool_results() -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    if let Ok(mut g) = TOOL_RESULT.lock() {
        if let Some(r) = g.take() {
            out.push((r.label, r.ok, r.note));
        }
    }
    out
}
