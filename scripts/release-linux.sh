#!/usr/bin/env bash
# Build Linux release artifacts into dist/:
#   dist/Hush-<ver>-amd64.deb  and  dist/Hush-<ver>-x86_64.AppImage
# Usage: scripts/release-linux.sh 0.1.5
#
# The binary must be built against glibc <= 2.39 (Ubuntu 24.04) to run there.
# If the host glibc is newer, the build runs inside an Ubuntu 24.04 chroot
# at $NOBLE_ROOT (default /opt/noble, created with debootstrap; see docs/LINUX.md).
set -euo pipefail
VER="${1:?usage: release-linux.sh <version>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"; mkdir -p "$DIST"
NOBLE_ROOT="${NOBLE_ROOT:-/opt/noble}"
MAX_GLIBC="2.39"

# Temporarily stamp the version (restored on exit; release script owns the real bump).
cp "$ROOT/Cargo.toml" /tmp/hush-Cargo.toml.bak; cp "$ROOT/Cargo.lock" /tmp/hush-Cargo.lock.bak
trap 'cp /tmp/hush-Cargo.toml.bak "$ROOT/Cargo.toml"; cp /tmp/hush-Cargo.lock.bak "$ROOT/Cargo.lock"' EXIT
sed -i "0,/^version = \".*\"/s//version = \"$VER\"/" "$ROOT/Cargo.toml"

# .cargo/config.toml pins MSVC-style *_FLAGS_RELEASE for Windows; real env vars win over it.
export CMAKE_C_FLAGS="-O3 -DNDEBUG" CMAKE_CXX_FLAGS="-O3 -DNDEBUG"
export CMAKE_C_FLAGS_RELEASE="-O3 -DNDEBUG" CMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG"
host_glibc="$(ldd --version | awk 'NR==1' | grep -oE '[0-9]+\.[0-9]+$')"
if [ "$(printf '%s\n%s\n' "$host_glibc" "$MAX_GLIBC" | sort -V | awk "END{print}")" = "$MAX_GLIBC" ]; then
  cargo build --release --manifest-path "$ROOT/Cargo.toml" --target-dir "$ROOT/target"
  BIN="$ROOT/target/release/hush-voice"
else
  echo "host glibc $host_glibc > $MAX_GLIBC: building in $NOBLE_ROOT"
  sudo mkdir -p "$NOBLE_ROOT/src"
  mountpoint -q "$NOBLE_ROOT/src" || sudo mount --bind "$ROOT" "$NOBLE_ROOT/src"
  mountpoint -q "$NOBLE_ROOT/proc" || sudo mount --bind /proc "$NOBLE_ROOT/proc"
  mountpoint -q "$NOBLE_ROOT/dev" || sudo mount --bind /dev "$NOBLE_ROOT/dev"
  sudo chroot "$NOBLE_ROOT" bash -c ". /root/.cargo/env && cd /src && \
    CMAKE_C_FLAGS='$CMAKE_C_FLAGS' CMAKE_CXX_FLAGS='$CMAKE_CXX_FLAGS' \
    CMAKE_C_FLAGS_RELEASE='$CMAKE_C_FLAGS' CMAKE_CXX_FLAGS_RELEASE='$CMAKE_CXX_FLAGS' \
    cargo build --release --target-dir /src/target-noble"
  BIN="$ROOT/target-noble/release/hush-voice"
fi

max="$(objdump -T "$BIN" | grep -oE 'GLIBC_[0-9.]+' | sed 's/GLIBC_//' | sort -V | awk "END{print}")"
echo "binary needs glibc $max"
[ "$(printf '%s\n%s\n' "$max" "$MAX_GLIBC" | sort -V | awk "END{print}")" = "$MAX_GLIBC" ] || { echo "glibc too new"; exit 1; }

# ---- staging tree (shared by .deb and AppImage) ----
STAGE="$(mktemp -d)"; PKG="$STAGE/deb"
install -Dm755 "$BIN" "$PKG/usr/bin/hush-voice"
strip "$PKG/usr/bin/hush-voice"
install -Dm644 "$ROOT/packaging/linux/hush-voice.desktop" "$PKG/usr/share/applications/hush-voice.desktop"
install -Dm644 "$ROOT/packaging/linux/hush-voice.svg" "$PKG/usr/share/icons/hicolor/scalable/apps/hush-voice.svg"
for s in 16 24 32 48 64 128 256 512; do
  d="$PKG/usr/share/icons/hicolor/${s}x${s}/apps"; mkdir -p "$d"
  rsvg-convert -w $s -h $s "$ROOT/packaging/linux/hush-voice.svg" -o "$d/hush-voice.png"
done

# ---- .deb ----
SIZE_KB="$(du -sk "$PKG/usr" | cut -f1)"
mkdir -p "$PKG/DEBIAN"
cat > "$PKG/DEBIAN/control" <<CTRL
Package: hush-voice
Version: $VER
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Jon <kushta.joni@gmail.com>
Installed-Size: $SIZE_KB
Depends: libc6 (>= 2.35), libgcc-s1, libstdc++6, libgtk-3-0t64 | libgtk-3-0, libayatana-appindicator3-1, libasound2t64 | libasound2, libx11-6, libx11-xcb1, libxcb1, libxtst6, libxdo3, libxkbcommon0, libxkbcommon-x11-0, libfontconfig1, libfreetype6, libvulkan1, mesa-vulkan-drivers, xdotool
Recommends: xclip
Homepage: https://github.com/ijon007/hush-voice
Description: Hush - local voice dictation
 Hold a shortcut, speak, and Hush types the text into any app.
 Transcription runs locally with Whisper; nothing leaves your machine.
CTRL
cat > "$PKG/DEBIAN/postinst" <<'POST'
#!/bin/sh
set -e
command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -f /usr/share/icons/hicolor || true
POST
chmod 755 "$PKG/DEBIAN/postinst"; cp "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/postrm"
fakeroot dpkg-deb --build -Zxz "$PKG" "$DIST/Hush-$VER-amd64.deb" 2>/dev/null || dpkg-deb --root-owner-group --build -Zxz "$PKG" "$DIST/Hush-$VER-amd64.deb"

# ---- AppImage (bundles only the binary; uses the system's GTK/X11 libs) ----
APPDIR="$STAGE/Hush.AppDir"
mkdir -p "$APPDIR"; cp -a "$PKG/usr" "$APPDIR/usr"
cp "$ROOT/packaging/linux/hush-voice.desktop" "$APPDIR/"
cp "$PKG/usr/share/icons/hicolor/256x256/apps/hush-voice.png" "$APPDIR/hush-voice.png"
cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/hush-voice" "$@"
RUN
chmod 755 "$APPDIR/AppRun"
AIT="${APPIMAGETOOL:-$ROOT/target/appimagetool-x86_64.AppImage}"
if [ ! -x "$AIT" ]; then
  curl -fsSL -o "$AIT" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$AIT"
fi
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$AIT" -n "$APPDIR" "$DIST/Hush-$VER-x86_64.AppImage"
rm -rf "$STAGE"
ls -la "$DIST"
