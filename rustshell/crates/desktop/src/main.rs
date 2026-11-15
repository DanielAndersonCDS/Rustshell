//! Desktop: full-screen borderless window pinned to the bottom of the z-order,
//! showing the wallpaper and the icons from the user's Desktop folder.
#![windows_subsystem = "windows"]

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Sense, TextureHandle};
use std::{path::PathBuf, process::Command};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

// Default wallpaper, baked into the exe.
static WALLPAPER: &[u8] = include_bytes!("../../../assets/wallpaper.png");

struct Desktop {
    wallpaper: Option<TextureHandle>,
    items: Vec<PathBuf>,
    selected: Option<usize>,
    placed: bool,
}

impl Desktop {
    fn new(ctx: &egui::Context) -> Self {
        let wallpaper = image::load_from_memory(WALLPAPER).ok().map(|img| {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            ctx.load_texture(
                "wallpaper",
                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        });
        let mut items: Vec<PathBuf> = dirs::desktop_dir()
            .and_then(|d| std::fs::read_dir(d).ok())
            .map(|rd| rd.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        items.retain(|p| {
            p.file_name().map_or(false, |n| !n.to_string_lossy().eq_ignore_ascii_case("desktop.ini"))
        });
        items.sort_by_key(|p| (!p.is_dir(), p.file_name().map(|n| n.to_ascii_lowercase())));
        Self { wallpaper, items, selected: None, placed: false }
    }
}

fn open(path: &PathBuf) {
    if path.is_dir() {
        // Prefer our own file manager, fall back to Explorer.
        if Command::new("rsfiles.exe").arg(path).spawn().is_err() {
            let _ = Command::new("explorer").arg(path).spawn();
        }
    } else {
        let _ = Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn();
    }
}

impl eframe::App for Desktop {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // First frame: cover the whole screen (physical px -> logical points).
        if !self.placed {
            self.placed = true;
            let ppp = ctx.pixels_per_point();
            let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN) as f32, GetSystemMetrics(SM_CYSCREEN) as f32) };
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2(0.0, 0.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w / ppp, h / ppp)));
        }

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let screen = ui.max_rect();
            let painter = ui.painter();

            // Wallpaper, "cover" fit (fills the screen, crops overflow).
            if let Some(tex) = &self.wallpaper {
                let img = tex.size_vec2();
                let (ia, sa) = (img.x / img.y, screen.width() / screen.height());
                let uv = if sa > ia {
                    let f = ia / sa;
                    Rect::from_min_max(pos2(0.0, (1.0 - f) / 2.0), pos2(1.0, (1.0 + f) / 2.0))
                } else {
                    let f = sa / ia;
                    Rect::from_min_max(pos2((1.0 - f) / 2.0, 0.0), pos2((1.0 + f) / 2.0, 1.0))
                };
                painter.image(tex.id(), screen, uv, Color32::WHITE);
            } else {
                painter.rect_filled(screen, 0.0, Color32::from_rgb(20, 60, 90));
            }

            // Icons, laid out top-to-bottom then left-to-right.
            let cell = vec2(88.0, 92.0);
            let rows = (((screen.height() - 70.0) / cell.y).floor() as usize).max(1);
            let mut open_idx = None;
            let mut clicked_any = false;
            for (i, path) in self.items.iter().enumerate() {
                let (col, row) = (i / rows, i % rows);
                let rect = Rect::from_min_size(
                    screen.min + vec2(12.0 + col as f32 * cell.x, 12.0 + row as f32 * cell.y),
                    cell,
                );
                let resp = ui.interact(rect, ui.id().with(i), Sense::click());
                if resp.clicked() { self.selected = Some(i); clicked_any = true; }
                if resp.double_clicked() { open_idx = Some(i); }
                let painter = ui.painter();
                if self.selected == Some(i) {
                    painter.rect_filled(rect, 4.0, Color32::from_white_alpha(45));
                } else if resp.hovered() {
                    painter.rect_filled(rect, 4.0, Color32::from_white_alpha(22));
                }
                let icon = if path.is_dir() { "📁" } else { "📄" };
                painter.text(rect.center_top() + vec2(0.0, 8.0), Align2::CENTER_TOP, icon, FontId::proportional(34.0), Color32::WHITE);
                let mut name = path.file_stem().or(path.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if name.chars().count() > 12 { name = name.chars().take(11).collect::<String>() + "…"; }
                let pos = rect.center_top() + vec2(0.0, 56.0);
                painter.text(pos + vec2(1.0, 1.0), Align2::CENTER_TOP, &name, FontId::proportional(13.0), Color32::from_black_alpha(200));
                painter.text(pos, Align2::CENTER_TOP, &name, FontId::proportional(13.0), Color32::WHITE);
            }
            if let Some(i) = open_idx { open(&self.items[i]); }
            // Click on empty space clears selection.
            if ui.input(|i| i.pointer.primary_clicked()) && !clicked_any { self.selected = None; }
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustShell Desktop")
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom)
            .with_position([0.0, 0.0])
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native("RustShell Desktop", options, Box::new(|cc| Ok(Box::new(Desktop::new(&cc.egui_ctx)))))
}
