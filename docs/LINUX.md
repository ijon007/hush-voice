# Hush on Linux

Hush keeps one codebase. Only the system layer (the part that talks to the OS) is split by platform, using `#[cfg(windows)]` / `#[cfg(target_os = "linux")]`. The Windows code was moved, not rewritten. Its behavior is unchanged.

## Layout
- `src/platform/linux.rs` holds all the Linux system calls:
  - X11 keyboard state (XQueryKeymap)
  - EWMH window helpers (active window, activate, own windows by `_NET_WM_PID`, map/unmap, overlay placement)
  - XTest Ctrl+V paste
  - XDG autostart
  - `xdg-open`
  - WAV playback (paplay / pw-play / aplay)
- Modules with Linux branches:
  - `hotkey.rs` (modifier and chord polling)
  - `paste.rs` (clipboard via GPUI, then refocus the target, release modifiers, Ctrl+V)
  - `tray.rs` (tray-icon on a GTK thread; menu items "Open Hush" and "Quit")
  - `overlay.rs` (pill on the display under the cursor, transparent, no WM frame)
  - `prefs.rs` (autostart)
  - `theme.rs` (data dir `~/.local/share/hush-voice`)
  - `sounds.rs`, `crashlog.rs`, `update.rs` (opens the release page; no installer)
- The main window uses client-side decorations on Linux, and the titlebar drag calls `start_window_move`.

## Linux-only crates
`x11` (xlib, xtest), `dirs`, `gtk` 0.18 (for the tray-icon event loop).

## Build deps (Debian/Ubuntu)
```
sudo apt install build-essential cmake clang pkg-config libx11-dev libxtst-dev \
  libxkbcommon-x11-dev libxcb1-dev libgtk-3-dev libayatana-appindicator3-dev \
  libasound2-dev libssl-dev
```
Optional runtime tools: `xdotool` (paste/activate fallback), `wtype` (Wayland paste), `paplay`/`pw-play`.

### Release builds
`.cargo/config.toml` sets MSVC-only CMAKE flags for whisper. On Linux, override them when building:
```
CMAKE_C_FLAGS="-O3 -DNDEBUG" CMAKE_CXX_FLAGS="-O3 -DNDEBUG" cargo build --release
```

## X11 vs Wayland
X11 is fully supported: global hotkey (grab), push-to-talk polling, auto-paste, tray, and pill placement.

Under Wayland, Hush runs through XWayland and degrades gracefully:
- Global hotkeys and key polling only see XWayland windows, so the chord may not fire while a native Wayland app has focus. Use a compositor shortcut instead.
- Auto-paste into native Wayland apps uses `wtype` if it is installed. Otherwise the text stays on the clipboard for a manual paste.
- Overlay positioning and window activation are up to the compositor.
- Tray needs a StatusNotifier host. On GNOME that means the AppIndicator extension.

## Known limits
- Left-clicking the tray icon opens the menu (appindicator has no click event). Use "Open Hush".
- No in-place auto-update. The update check opens the GitHub release page.

## Packaging (later)
- **AppImage:** bundle the binary plus assets with linuxdeploy (gtk plugin). Autostart already respects `$APPIMAGE`.
- **.deb:** `cargo-deb`, with a `.desktop` file and icons in `/usr/share`, and the deps above as `Depends`.

## Packaging (.deb / AppImage)

`scripts/release-linux.sh <version>` builds `dist/Hush-<ver>-amd64.deb` and
`dist/Hush-<ver>-x86_64.AppImage`. The binary must target glibc <= 2.39 (Ubuntu 24.04);
on newer hosts the script builds inside an Ubuntu 24.04 chroot at `/opt/noble`:

    sudo debootstrap --variant=minbase --include=ca-certificates,curl noble /opt/noble http://archive.ubuntu.com/ubuntu
    # then inside: universe repo, build deps (see README build command + libxdo-dev), rustup

Needs on the host: rsvg-convert (librsvg2-bin), dpkg-deb, fakeroot, file, objdump.
The .deb pulls runtime deps (GTK3, ayatana-appindicator, ALSA, xdotool, …) on install.
The AppImage bundles only the binary and relies on those system libs being present.
