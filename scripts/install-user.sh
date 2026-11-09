#!/usr/bin/env bash
# Install a source build on Linux without root; remove it with --uninstall.
set -euo pipefail

usage() {
    echo "Usage: $0 [--uninstall] [--prefix DIRECTORY]"
    echo 'Installs target/release/{inkboard,ink,inkboard-mcp} under ~/.local by default.'
}

prefix="${HOME:?HOME is required}/.local"
uninstall=false
while (($#)); do
    case "$1" in
        --uninstall) uninstall=true; shift ;;
        --prefix)
            if (($# < 2)) || [[ -z "$2" ]]; then usage >&2; exit 2; fi
            prefix=$2; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done
[[ $(uname -s) == Linux ]] || { echo 'This installer requires Linux.' >&2; exit 1; }
[[ "$prefix" == /* && "$prefix" != *$'\n'* && "$prefix" != *$'\r'* ]] || {
    echo 'The prefix must be an absolute path without line breaks.' >&2; exit 2;
}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
bin="$prefix/bin"
share="$prefix/share"
desktop="$share/applications/inkboard.desktop"
icon="$share/icons/hicolor/512x512/apps/inkboard.png"
mime="$share/mime/packages/inkboard.xml"
doc="$share/doc/inkboard"

if "$uninstall"; then
    rm -f -- "$bin/inkboard" "$bin/ink" "$bin/inkboard-mcp" "$desktop" "$icon" "$mime"
    for file in README.md LICENSE-MIT LICENSE-APACHE THIRD_PARTY_LICENSES.md; do
        rm -f -- "$doc/$file"
    done
    rmdir -- "$doc" 2>/dev/null || true
else
    # Check the entire payload before changing an existing installation.
    for file in inkboard ink inkboard-mcp; do
        [[ -x "$repo/target/release/$file" ]] || {
            echo 'Build first: cargo build --locked --release -p inkboard-app -p inkboard-cli -p inkboard-mcp' >&2
            exit 1
        }
    done
    for file in inkboard ink inkboard-mcp; do
        install -Dm755 -- "$repo/target/release/$file" "$bin/$file"
    done
    install -Dm644 -- "$repo/crates/inkboard-app/assets/icon.png" "$icon"
    install -Dm644 -- "$repo/packaging/linux/inkboard.xml" "$mime"
    for file in README.md LICENSE-MIT LICENSE-APACHE THIRD_PARTY_LICENSES.md; do
        install -Dm644 -- "$repo/$file" "$doc/$file"
    done
    mkdir -p -- "$(dirname -- "$desktop")"
    # Desktop Entry string escaping is applied after Exec quoting: backslashes need four copies.
    exe=${bin//\\/\\\\\\\\}/inkboard
    exe=${exe//\"/\\\\\"}
    exe=${exe//\$/\\\\\$}
    exe=${exe//\`/\\\\\`}
    exe=${exe//%/%%}
    cat > "$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Inkboard
Comment=An infinite, offline whiteboard
Exec=/usr/bin/env "$exe" %u
Icon=inkboard
Terminal=false
Categories=Graphics;
MimeType=application/x-inkboard;x-scheme-handler/inkboard;
EOF
    chmod 644 -- "$desktop"
fi

if command -v update-desktop-database >/dev/null && [[ -d "$share/applications" ]]; then
    update-desktop-database "$share/applications"
fi
if command -v update-mime-database >/dev/null && [[ -d "$share/mime" ]]; then
    update-mime-database "$share/mime"
fi
if command -v gtk-update-icon-cache >/dev/null && [[ -d "$share/icons/hicolor" ]]; then
    gtk-update-icon-cache --force --ignore-theme-index "$share/icons/hicolor"
fi
if "$uninstall"; then
    echo 'Inkboard removed. Saved boards and application data were preserved.'
else
    echo "Installed to $prefix. Launch $bin/inkboard or use the application menu."
    echo "Add $bin to PATH to use ink and inkboard-mcp from a terminal."
    echo "Uninstall with: $0 --uninstall --prefix \"$prefix\""
fi
