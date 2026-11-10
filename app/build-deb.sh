#!/usr/bin/env bash
# Build the .deb package for Ferry摇摆人 (Linux-first rebuild).
# Produces: deb/ferry_<version>_amd64.deb
set -euo pipefail

VERSION=0.1.0
PKG=ferry
STAGE=deb/${PKG}_${VERSION}_amd64
ARCH=$(dpkg --print-architecture)

rm -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" \
         "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/256x256/apps" \
         "$STAGE/usr/share/doc/$PKG"

# binary (built by cargo build --release)
cp target/release/ferry "$STAGE/usr/bin/ferry"
chmod 755 "$STAGE/usr/bin/ferry"

# control file
cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Depends: ffmpeg, zenity, xclip
Conflicts: fffactory
Replaces: fffactory
Installed-Size: $(du -sk "$STAGE/usr" | cut -f1)
Maintainer: Ferry Project <rebuild@localhost>
Description: Ferry摇摆人（独立重建版）
 多媒体格式转换工具：视频、音频、图片互转，支持合并与混流。
 界面简洁，Linux 优先，转换由 ffmpeg 完成，图片转换内置解码。
 本应用为独立重建，不包含任何原始安装包中的代码。
EOF

# desktop entry
cat > "$STAGE/usr/share/applications/ferry.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Ferry摇摆人
GenericName=媒体格式转换
Comment=视频、音频、图片格式转换，支持合并与混流
Exec=/usr/bin/ferry
Icon=ferry
StartupWMClass=ferry
Terminal=false
Categories=AudioVideo;Video;Audio;Utility;Graphics;
Keywords=convert;converter;ffmpeg;Ferry;ferry;转换;格式;摇摆人;
EOF

# copyright
cat > "$STAGE/usr/share/doc/$PKG/copyright" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: ferry
Source: local rebuild

Files: *
Copyright: 2026 Ferry Project
License: MIT
 This rebuild is original code written for study purposes.
 It contains no code from the original Windows application.
EOF

# icon (the product mark is kept in deb/icon.png, 256x256 PNG)
cp deb/icon.png "$STAGE/usr/share/icons/hicolor/256x256/apps/ferry.png"

# refresh icon and desktop caches so panels pick up the running-app icon
cat > "$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
exit 0
EOF
chmod 755 "$STAGE/DEBIAN/postinst"

cat > "$STAGE/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
exit 0
EOF
chmod 755 "$STAGE/DEBIAN/postrm"

# build
dpkg-deb --build --root-owner-group "$STAGE"
echo "OK: $STAGE.deb"
