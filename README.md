# Hush

Hush is a private voice-dictation app for Windows. Hold **Ctrl + Space**, speak,
let go, and your words are typed into whatever app you're using. Speech
recognition runs entirely on your PC with [whisper.cpp](https://github.com/ggerganov/whisper.cpp);
no audio leaves your machine.

## Features

- **Hold-to-talk anywhere.** Ctrl + Space shows a small recording pill; release to
  transcribe and paste into the focused app (or copy to the clipboard only).
- **On-device models.** Fast, Balanced and Best whisper models, downloaded from
  Settings, or point Hush at your own whisper.cpp `.bin`.
- **Styles.** Formal (fully punctuated), Casual (capitalized, light punctuation) or
  Lowercase. Filler words like "um" and "uh" are always removed.
- **Dictionary and Shortcuts.** Teach Hush names and jargon; expand spoken
  phrases ("my email") into text.
- **History and stats.** Recent dictations with one-click copy, words dictated,
  speaking speed and streak. Keep history forever, 30 days or 7 days.
- **Settings.** Launch at login, sound cues, microphone choice, overlay position,
  light/dark theme, update check, crash logs.

## Install

Download `Hush-Setup-X.Y.Z.exe` from the
[latest release](https://github.com/ijon007/hush-voice/releases/latest) and run it.
It installs for your user only (no admin prompt) and adds a Start menu shortcut;
uninstall from Windows Settings > Apps. Open Hush, go to **Settings > Dictation**
and download a model to get started.

Hush checks GitHub for a newer release on launch and shows "Update available"
in the title bar when there is one.

## Build from source

Requirements (Windows 10/11, x64):

- [Rust](https://rustup.rs/) (stable, MSVC toolchain)
- Visual Studio 2022 with **Desktop development with C++**
- [LLVM](https://llvm.org/) (`winget install LLVM.LLVM`) for whisper-rs bindings;
  set `LIBCLANG_PATH=C:\Program Files\LLVM\bin`
- [CMake](https://cmake.org/) (`winget install Kitware.CMake`)

whisper.cpp is compiled by `whisper-rs-sys`, which needs the MSVC environment.
Build from an **x64 Native Tools** prompt, or wrap the command with `vcvars64.bat`:

```powershell
cmd /c '"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul && cargo build'
.\target\debug\hush-voice.exe
```

Run the tests with `cargo test` (same environment).

Debug helpers (no window):

```powershell
hush-voice.exe --download fast                        # fetch a model
hush-voice.exe --transcribe clip.wav balanced en      # transcribe a WAV file
```

App data lives in `%APPDATA%\hush-voice` (settings, history, models, logs).

## Releasing

Releases are made by hand from a Windows machine with the build tools above,
[Inno Setup 6](https://jrsoftware.org/isinfo.php) (`winget install JRSoftware.InnoSetup`)
and the [GitHub CLI](https://cli.github.com/) (`winget install GitHub.cli`, then `gh auth login` once).

```powershell
.\scripts\release.ps1                 # release the version already in Cargo.toml
.\scripts\release.ps1 -Version 0.2.0  # bump Cargo.toml, commit "Release v0.2.0", then release
.\scripts\release.ps1 -Draft          # same, but the GitHub release starts as a draft
```

The script needs a clean working tree. It builds `cargo build --release` inside the
MSVC environment, packages `installer\Output\Hush-Setup-X.Y.Z.exe` with
`installer\hush.iss`, tags `vX.Y.Z`, pushes the commit and tag, and creates the
GitHub release with the installer attached and generated notes. Installed copies of
Hush see the new release on their next launch ("Update available").

## License

[MIT](LICENSE) (c) ijon007
