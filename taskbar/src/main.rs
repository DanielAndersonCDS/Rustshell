//! Taskbar: an AppBar docked to the bottom edge (reserves screen space so
//! maximized windows stop above it), with Start button, open-window buttons
//! and a clock.
#![windows_subsystem = "windows"]

use eframe::egui::{self, pos2, vec2, Color32, RichText};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{mem::size_of, process::Command, time::Duration};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    UI::{
        Shell::{SHAppBarMessage, ABE_BOTTOM, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA},
        WindowsAndMessaging::*,
    },
};

const BAR_HEIGHT_PT: f32 = 44.0;
const OWN_TITLES: [&str; 2] = ["RustShell Taskbar", "RustShell Desktop"];

struct TaskWindow { hwnd: HWND, title: String }

struct Taskbar {
    hwnd: HWND,
    rect: RECT, // physical pixels, as granted by the shell
    placed: bool,
    windows: Vec<TaskWindow>,
}

fn register_appbar(hwnd: HWND, height_px: i32) -> RECT {
    unsafe {
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let mut abd = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uEdge: ABE_BOTTOM,
            rc: RECT { left: 0, top: sh - height_px, right: sw, bottom: sh },
            ..Default::default()
        };
        SHAppBarMessage(ABM_NEW, &mut abd);
        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        abd.rc.top = abd.rc.bottom - height_px;
        SHAppBarMessage(ABM_SETPOS, &mut abd);
        abd.rc
    }
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<TaskWindow>);
    if !IsWindowVisible(hwnd).as_bool() { return true.into(); }
    // Skip owned windows and tool windows (they don't get taskbar buttons).
    if GetWindow(hwnd, GW_OWNER).is_ok() { return true.into(); }
    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if ex & WS_EX_TOOLWINDOW.0 != 0 { return true.into(); }
    // Skip cloaked windows (suspended UWP apps, other virtual desktops).
    let mut cloaked = 0u32;
    let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
    if cloaked != 0 { return true.into(); }
    let mut buf = [0u16; 256];
    let n = GetWindowTextW(hwnd, &mut buf) as usize;
    if n == 0 { return true.into(); }
    let title = String::from_utf16_lossy(&buf[..n]);
    if title == "Program Manager" || OWN_TITLES.contains(&title.as_str()) { return true.into(); }
    out.push(TaskWindow { hwnd, title });
    true.into()
}

fn list_windows() -> Vec<TaskWindow> {
    let mut v: Vec<TaskWindow> = Vec::new();
    unsafe { let _ = EnumWindows(Some(enum_cb), LPARAM(&mut v as *mut _ as isize)); }
    v
}

fn activate(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() { let _ = ShowWindow(hwnd, SW_RESTORE); }
        let _ = SetForegroundWindow(hwnd);
    }
}

impl eframe::App for Taskbar {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if !self.placed {
            self.placed = true;
            let ppp = ctx.pixels_per_point();
            let r = self.rect;
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2(r.left as f32 / ppp, r.top as f32 / ppp)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
                (r.right - r.left) as f32 / ppp,
                (r.bottom - r.top) as f32 / ppp,
            )));
        }
        self.windows = list_windows(); // cheap enough at 1 Hz
        let fg = unsafe { GetForegroundWindow() };

        let frame = egui::Frame::none().fill(Color32::from_rgb(24, 26, 32)).inner_margin(egui::Margin::symmetric(6.0, 4.0));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let start = egui::Button::new(RichText::new("⊞  Start").size(15.0).strong())
                    .min_size(vec2(84.0, BAR_HEIGHT_PT - 12.0));
                if ui.add(start).clicked() {
                    // Placeholder: real Start menu comes later (needs its own popup window).
                    let _ = Command::new("rsfiles.exe").spawn();
                }
                ui.separator();
                for w in &self.windows {
                    let mut t = w.title.clone();
                    if t.chars().count() > 22 { t = t.chars().take(21).collect::<String>() + "…"; }
                    let b = egui::Button::new(t)
                        .selected(w.hwnd == fg)
                        .min_size(vec2(150.0, BAR_HEIGHT_PT - 12.0));
                    if ui.add(b).on_hover_text(&w.title).clicked() { activate(w.hwnd); }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let now = chrono::Local::now();
                    ui.label(RichText::new(format!("{}\n{}", now.format("%H:%M"), now.format("%d/%m/%Y"))).size(12.0));
                });
            });
        });
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let mut abd = APPBARDATA { cbSize: size_of::<APPBARDATA>() as u32, hWnd: self.hwnd, ..Default::default() };
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut abd); }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustShell Taskbar")
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_always_on_top()
            .with_inner_size([1280.0, BAR_HEIGHT_PT]),
        ..Default::default()
    };
    eframe::run_native("RustShell Taskbar", options, Box::new(|cc| {
        let hwnd = match cc.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => HWND(h.hwnd.get() as *mut _),
            _ => panic!("not a Win32 window"),
        };
        let height_px = (BAR_HEIGHT_PT * cc.egui_ctx.pixels_per_point()).round() as i32;
        let rect = register_appbar(hwnd, height_px);
        Ok(Box::new(Taskbar { hwnd, rect, placed: false, windows: vec![] }))
    }))
}
