#!/bin/sh
# TraceLens installer for Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/UltraSive/tracelens/main/install.sh | sh
#
# Environment:
#   TRACELENS_VERSION=v0.1.0   install a specific release (default: latest)
#   TRACELENS_METHOD=appimage  Linux: skip the .deb/.rpm and install the extracted AppImage
#
# Windows: download the installer from https://github.com/UltraSive/tracelens/releases
set -eu

REPO="UltraSive/tracelens"

say() { printf '%s\n' "$*"; }
err() { printf 'tracelens-install: %s\n' "$*" >&2; exit 1; }
has() { command -v "$1" >/dev/null 2>&1; }

sudo_cmd() {
	if [ "$(id -u)" -eq 0 ]; then
		"$@"
	elif has sudo; then
		sudo "$@"
	else
		err "need root for: $* (install sudo or run as root)"
	fi
}

download() {
	say "Downloading $1"
	curl -fL --progress-bar -o "$2" "$1" || err "download failed: $1"
}

resolve_version() {
	if [ -n "${TRACELENS_VERSION:-}" ]; then
		case "$TRACELENS_VERSION" in
			v*) echo "$TRACELENS_VERSION" ;;
			*) echo "v$TRACELENS_VERSION" ;;
		esac
		return
	fi
	# Follow the /releases/latest redirect instead of the API (no rate limit).
	url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") ||
		err "could not reach GitHub"
	tag=${url##*/}
	case "$tag" in
		v*) echo "$tag" ;;
		*) err "no published release found" ;;
	esac
}

install_linux() {
	case "$(uname -m)" in
		x86_64 | amd64) ;;
		*) err "only x86_64 Linux builds are published; build from source for $(uname -m)" ;;
	esac

	method=${TRACELENS_METHOD:-}
	if [ -z "$method" ]; then
		if has apt-get && has dpkg; then
			method=deb
		elif has dnf || has zypper || has yum; then
			method=rpm
		else
			method=appimage
		fi
	fi

	case "$method" in
		deb)
			file="$tmp/TraceLens_${ver}_amd64.deb"
			download "$base/TraceLens_${ver}_amd64.deb" "$file"
			# apt resolves the WebKitGTK dependencies; postinst grants CAP_NET_RAW.
			sudo_cmd apt-get install -y "$file"
			;;
		rpm)
			file="$tmp/TraceLens-${ver}-1.x86_64.rpm"
			download "$base/TraceLens-${ver}-1.x86_64.rpm" "$file"
			if has dnf; then
				sudo_cmd dnf install -y "$file"
			elif has zypper; then
				sudo_cmd zypper --non-interactive install --allow-unsigned-rpm "$file"
			else
				sudo_cmd yum install -y "$file"
			fi
			;;
		appimage)
			install_appimage
			;;
		*) err "unknown TRACELENS_METHOD '$method' (deb, rpm or appimage)" ;;
	esac
}

# Capabilities don't survive the AppImage's FUSE mount (nosuid), so extract it
# and grant CAP_NET_RAW to the real binary instead.
install_appimage() {
	data="${XDG_DATA_HOME:-$HOME/.local/share}"
	dest="$data/tracelens"
	bindir="$HOME/.local/bin"
	file="$tmp/TraceLens.AppImage"
	download "$base/TraceLens_${ver}_amd64.AppImage" "$file"
	chmod +x "$file"
	(cd "$tmp" && "$file" --appimage-extract >/dev/null) || err "could not extract the AppImage"

	rm -rf "$dest"
	mkdir -p "$data" "$bindir" "$data/applications"
	mv "$tmp/squashfs-root" "$dest"
	# AppRun locates its files from $0, so call it through a wrapper, not a symlink.
	rm -f "$bindir/tracelens"
	printf '#!/bin/sh\nexec "%s/AppRun" "$@"\n' "$dest" >"$bindir/tracelens"
	chmod +x "$bindir/tracelens"

	icon="$dest/tracelens.png"
	[ -f "$icon" ] || icon=$(find "$dest" -name '*.png' -path '*tracelens*' 2>/dev/null | head -n 1)
	cat >"$data/applications/tracelens.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=TraceLens
Comment=Network path diagnostics (MTR-style traceroute)
Exec=$bindir/tracelens %U
Icon=$icon
Categories=Network;Utility;
Terminal=false
EOF

	if has setcap || [ -x /usr/sbin/setcap ] || [ -x /sbin/setcap ]; then
		setcap_bin=$(command -v setcap || ls /usr/sbin/setcap /sbin/setcap 2>/dev/null | head -n 1)
		say "Granting raw-socket permission (CAP_NET_RAW) to the binary; sudo may ask for your password."
		sudo_cmd "$setcap_bin" cap_net_raw+ep "$dest/usr/bin/tracelens" ||
			say "warning: setcap failed; tracing will need root"
	else
		say "warning: setcap not found (install libcap2-bin / libcap); then run:"
		say "  sudo setcap cap_net_raw+ep $dest/usr/bin/tracelens"
	fi

	case ":$PATH:" in
		*":$bindir:"*) ;;
		*) say "note: add $bindir to your PATH to run 'tracelens' from a terminal" ;;
	esac
}

install_macos() {
	case "$(uname -m)" in
		arm64 | aarch64) arch=aarch64 ;;
		x86_64) arch=x64 ;;
		*) err "unsupported Mac architecture $(uname -m)" ;;
	esac

	if [ -w /Applications ]; then apps=/Applications; else apps="$HOME/Applications"; fi
	mkdir -p "$apps"

	file="$tmp/TraceLens.app.tar.gz"
	download "$base/TraceLens_${arch}.app.tar.gz" "$file"
	tar -xzf "$file" -C "$tmp"
	[ -d "$tmp/TraceLens.app" ] || err "archive did not contain TraceLens.app"

	rm -rf "$apps/TraceLens.app"
	mv "$tmp/TraceLens.app" "$apps/"
	# The build is unsigned; clear quarantine so Gatekeeper doesn't block the first launch.
	xattr -dr com.apple.quarantine "$apps/TraceLens.app" 2>/dev/null || true

	mkdir -p "$HOME/.local/bin"
	ln -sf "$apps/TraceLens.app/Contents/MacOS/tracelens" "$HOME/.local/bin/tracelens"
	say "Installed to $apps/TraceLens.app (CLI: ~/.local/bin/tracelens)"
}

main() {
	has curl || err "curl is required"
	ver_tag=$(resolve_version)
	ver=${ver_tag#v}
	base="https://github.com/$REPO/releases/download/$ver_tag"

	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT INT TERM

	say "Installing TraceLens $ver_tag"
	case "$(uname -s)" in
		Linux) install_linux ;;
		Darwin) install_macos ;;
		*) err "unsupported OS $(uname -s); for Windows download the installer from https://github.com/$REPO/releases" ;;
	esac
	say "Done. Launch TraceLens from your app menu, or run: tracelens example.com"
}

# Everything runs from main so a truncated download can't execute half a script.
main "$@"
