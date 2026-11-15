# RustShell

Three programs in one Cargo workspace:

| Binary        | What it does                                                        |
|---------------|---------------------------------------------------------------------|
| `rsdesktop`   | Full-screen wallpaper window + desktop icons                        |
| `rstaskbar`   | AppBar docked at the bottom: Start button, window buttons, clock    |
| `rsfiles`     | File manager (sidebar, address bar, back/forward/up, open)          |

## Build (on Windows, Rust stable + MSVC toolchain)

    cargo build --release

Binaries land in `target\release\`. Keep them in the same folder so they can launch each other.

## Run alongside Explorer (safe)

    target\release\rsdesktop.exe
    target\release\rstaskbar.exe
    target\release\rsfiles.exe

Explorer's own desktop/taskbar will still be present; ours are layered with it.
The wallpaper lives in `assets/wallpaper.png` and is embedded at compile time.

## Not done yet
- Start menu (needs its own popup window), system tray, notifications
- Taskbar window icons, pinned apps, multi-monitor
- Desktop: right-click menu, drag/drop, parenting under WorkerW
- File manager: copy/move/delete (IFileOperation), real icons, context menu, async loading
- Replacing Explorer as the shell (HKCU\...\Winlogon\Shell) - test in a VM first!
