#!/usr/bin/env bash
#
# Install a Linux Designer bundle on a *clean* image, launch it under a virtual
# display, and verify it stays alive. A clean image carries none of the build
# runner's GTK/WebKitGTK libraries, so an unresolved runtime dependency fails
# loudly here instead of on a user's machine.
#
# Two things are checked:
#   1. Install mechanics  — the package installs and its declared dependencies
#      resolve on a stock distro (apt/dnf do the resolution; AppImage carries
#      its own libs but still needs the host's system WebKitGTK stack).
#   2. Launch             — the app starts and is still running after ~15s.
#
# The "alive after 15s" check is a liveness proxy: it reliably catches a
# missing-library crash (those die immediately), but it cannot see a blank or
# non-interactive window. The root-window screenshot captured at the end is the
# artifact for spotting that by eye.
#
# Usage: run-smoke.sh <deb|rpm|AppImage> <bundle-path> <screenshot-path>
set -euo pipefail

format="$1"               # deb | rpm | AppImage
bundle="$(realpath "$2")" # the .deb/.rpm/.AppImage to test
shot="$3"                 # where to write the root-window screenshot (PNG)

# sudo only where it exists and we are not already root (minimal containers run
# as root with no sudo; the host runner needs sudo).
SUDO=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null; then
  SUDO="sudo"
fi

# ── Test prerequisites: a virtual X server + a screenshot tool ──────────────
# These are CI-harness tools, not part of what we're testing — installing them
# does not pre-seed the bundle's WebKitGTK dependencies.
if command -v apt-get >/dev/null; then
  PM=apt
  $SUDO apt-get update
  $SUDO apt-get install -y xvfb imagemagick
elif command -v dnf >/dev/null; then
  PM=dnf
  $SUDO dnf install -y xorg-x11-server-Xvfb ImageMagick
else
  echo "::error::no supported package manager (apt-get/dnf) found"
  exit 1
fi

# ── Install the bundle and resolve the launchable binary ────────────────────
case "$format" in
  deb)
    # An absolute path makes apt treat the argument as a local package and pull
    # in its declared dependencies; a missing runtime dep fails right here.
    $SUDO apt-get install -y "$bundle"
    bin=/usr/bin/mathdoku-designer
    ;;
  rpm)
    $SUDO dnf install -y "$bundle"
    bin=/usr/bin/mathdoku-designer
    ;;
  AppImage)
    # No FUSE on CI runners, so extract instead of mounting and run AppRun.
    chmod +x "$bundle"
    "$bundle" --appimage-extract >/dev/null
    bin="$PWD/squashfs-root/AppRun"
    ;;
  *)
    echo "::error::unknown bundle format: $format"
    exit 1
    ;;
esac

echo "Installed via $PM; launching $bin under Xvfb…"

# WebKitGTK under Xvfb with no GPU is finicky. Force software rendering and
# disable the dmabuf/compositing paths it cannot satisfy headless, or it
# crashes on startup for reasons unrelated to a missing dependency.
export LIBGL_ALWAYS_SOFTWARE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export BIN="$bin" SHOT="$shot"

# xvfb-run owns the virtual display; the inner shell launches the app, waits,
# checks liveness, then screenshots the root window on the same DISPLAY.
xvfb-run -a --server-args="-screen 0 1280x1024x24" bash -euo pipefail -c '
  "$BIN" &
  app=$!
  sleep 15
  if ! kill -0 "$app" 2>/dev/null; then
    echo "::error::$BIN exited within 15s — likely a missing runtime dependency"
    wait "$app" || true
    exit 1
  fi
  echo "$BIN still alive after 15s."
  import -window root "$SHOT" || echo "::warning::screenshot capture failed (non-fatal)"
  kill "$app" 2>/dev/null || true
'
