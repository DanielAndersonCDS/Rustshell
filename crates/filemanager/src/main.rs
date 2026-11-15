//! File manager MVP: sidebar (drives + quick links), address bar,
//! back/forward/up, sortable-by-default list (folders first), open on double-click.
#![windows_subsystem = "windows"]

use chrono::{DateTime, Local};
use eframe::egui;
use std::{fs, os::windows::fs::MetadataExt, path::PathBuf, process::Command, time::SystemTime};

struct Entry { name: String, path: PathBuf, is_dir: bool, size: u64, modified: Option<SystemTime> }

struct Fm {
    cwd: PathBuf,
    history: Vec<PathBuf>,
    pos: usize,
    entries: Vec<Entry>,
    selected: Option<usize>,
    addr: String,
    error: Option<String>,
    show_hidden: bool,
}

impl Fm {
    fn new(start: PathBuf) -> Self {
        let mut s = Self {
            cwd: start.clone(), history: vec![start], pos: 0, entries: vec![],
            selected: None, addr: String::new(), error: None, show_hidden: false,
        };
        s.load();
        s
    }

    fn load(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.error = None;
        self.addr = self.cwd.display().to_string();
        match fs::read_dir(&self.cwd) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let Ok(md) = e.metadata() else { continue };
                    let hidden = md.file_attributes() & 0x2 != 0;
                    if hidden && !self.show_hidden { continue; }
                    self.entries.push(Entry {
                        name: e.file_name().to_string_lossy().to_string(),
                        path: e.path(), is_dir: md.is_dir(), size: md.len(), modified: md.modified().ok(),
                    });
                }
                self.entries.sort_by_key(|e| (!e.is_dir, e.name.to_lowercase()));
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn go(&mut self, p: PathBuf) {
        if p == self.cwd { return; }
        self.history.truncate(self.pos + 1);
        self.history.push(p.clone());
        self.pos += 1;
        self.cwd = p;
        self.load();
    }
    fn back(&mut self) { if self.pos > 0 { self.pos -= 1; self.cwd = self.history[self.pos].clone(); self.load(); } }
    fn forward(&mut self) { if self.pos + 1 < self.history.len() { self.pos += 1; self.cwd = self.history[self.pos].clone(); self.load(); } }
    fn up(&mut self) { if let Some(p) = self.cwd.parent() { self.go(p.to_path_buf()); } }
}

fn human(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let (mut v, mut i) = (n as f64, 0);
    while v >= 1024.0 && i < units.len() - 1 { v /= 1024.0; i += 1; }
    if i == 0 { format!("{} B", n) } else { format!("{:.1} {}", v, units[i]) }
}

impl eframe::App for Fm {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut nav: Option<PathBuf> = None;

        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(self.pos > 0, egui::Button::new("⬅")).clicked() { self.back(); }
                if ui.add_enabled(self.pos + 1 < self.history.len(), egui::Button::new("➡")).clicked() { self.forward(); }
                if ui.button("⬆").clicked() { self.up(); }
                if ui.button("⟳").clicked() { self.load(); }
                let r = ui.add(egui::TextEdit::singleline(&mut self.addr).desired_width(ui.available_width() - 110.0));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let p = PathBuf::from(&self.addr);
                    if p.is_dir() { nav = Some(p); } else { self.addr = self.cwd.display().to_string(); }
                }
                if ui.checkbox(&mut self.show_hidden, "Hidden").changed() { self.load(); }
            });
        });

        egui::SidePanel::left("side").default_width(170.0).show(ctx, |ui| {
            ui.heading("Quick access");
            let links = [
                ("🖥 Desktop", dirs::desktop_dir()), ("📥 Downloads", dirs::download_dir()),
                ("📄 Documents", dirs::document_dir()), ("🖼 Pictures", dirs::picture_dir()),
                ("🎵 Music", dirs::audio_dir()), ("🎬 Videos", dirs::video_dir()),
            ];
            for (label, path) in links {
                if let Some(p) = path { if ui.selectable_label(false, label).clicked() { nav = Some(p); } }
            }
            ui.separator();
            ui.heading("This PC");
            for letter in b'A'..=b'Z' {
                let root = PathBuf::from(format!("{}:\\", letter as char));
                if root.exists() && ui.selectable_label(false, format!("💽 {}:", letter as char)).clicked() { nav = Some(root); }
            }
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let text = match self.selected.and_then(|i| self.entries.get(i)) {
                Some(e) => format!("{} selected", e.name),
                None => format!("{} items", self.entries.len()),
            };
            ui.label(text);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED, err); }
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                egui::Grid::new("files").num_columns(3).striped(true).min_col_width(80.0).spacing([24.0, 4.0]).show(ui, |ui| {
                    ui.strong("Name"); ui.strong("Modified"); ui.strong("Size"); ui.end_row();
                    for (i, e) in self.entries.iter().enumerate() {
                        let label = format!("{} {}", if e.is_dir { "📁" } else { "📄" }, e.name);
                        let r = ui.selectable_label(self.selected == Some(i), label);
                        if r.clicked() { self.selected = Some(i); }
                        if r.double_clicked() {
                            if e.is_dir { nav = Some(e.path.clone()); }
                            else { let _ = Command::new("cmd").args(["/C", "start", ""]).arg(&e.path).spawn(); }
                        }
                        ui.label(e.modified.map(|t| DateTime::<Local>::from(t).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default());
                        ui.label(if e.is_dir { String::new() } else { human(e.size) });
                        ui.end_row();
                    }
                });
            });
        });

        if let Some(p) = nav { self.go(p); }
    }
}

fn main() -> eframe::Result {
    let start = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.is_dir())
        .or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("C:\\"));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("RustShell Files").with_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native("RustShell Files", options, Box::new(|_| Ok(Box::new(Fm::new(start)))))
}
