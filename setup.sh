#!/usr/bin/env bash
# setup.sh — install libgringotts and its dependencies before building rgringotts
#
# Usage:
#   ./setup.sh                         # installs to /usr/local (may require sudo)
#   LIBGRINGOTTS_PREFIX=$HOME/.local ./setup.sh   # user-local, no sudo
#
# After a non-standard prefix, export before running cargo build:
#   export LIBGRINGOTTS_DIR=$HOME/.local
#   export LIBRARY_PATH=$HOME/.local/lib:$LIBRARY_PATH
#   export PKG_CONFIG_PATH=$HOME/.local/lib/pkgconfig:$PKG_CONFIG_PATH
set -euo pipefail

PREFIX="${LIBGRINGOTTS_PREFIX:-/usr/local}"
TMPDIR_BUILD="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_BUILD"' EXIT

log()  { printf '  \033[32m[setup]\033[0m %s\n' "$*"; }
warn() { printf '  \033[33m[setup]\033[0m %s\n' "$*"; }
die()  { printf '  \033[31m[setup] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

need() {
    command -v "$1" &>/dev/null || die "'$1' is required but not found — install it first"
}

# ── Already installed? ────────────────────────────────────────────────────────
already_installed() {
    if pkg-config --exists libgringotts 2>/dev/null; then
        log "libgringotts already found via pkg-config ($(pkg-config --modversion libgringotts))"
        return 0
    fi
    # Check common locations for the shared/static library
    local search_dirs="/usr/lib /usr/local/lib /opt/homebrew/lib ${PREFIX}/lib"
    for dir in $search_dirs; do
        for ext in so dylib a; do
            [ -f "$dir/libgringotts.$ext" ] && return 0
        done
    done
    return 1
}

# ── Helpers ───────────────────────────────────────────────────────────────────
ncpu() { sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4; }

maybe_sudo() {
    # Only use sudo if we're installing to a system directory
    [[ "$PREFIX" == /usr* ]] && echo sudo || echo ""
}

# ── Install libmcrypt (removed from Homebrew core) ───────────────────────────
build_libmcrypt() {
    if pkg-config --exists libmcrypt 2>/dev/null; then
        log "libmcrypt already installed"
        return
    fi
    for ext in so dylib a; do
        [ -f "${PREFIX}/lib/libmcrypt.$ext" ] && { log "libmcrypt already installed"; return; }
    done

    # Try the discoteq tap first — much faster than a source build
    log "Installing libmcrypt via Homebrew tap (discoteq/discoteq)..."
    if brew tap discoteq/discoteq &>/dev/null && brew install discoteq/discoteq/libmcrypt &>/dev/null; then
        log "libmcrypt installed via Homebrew"
        return
    fi
    warn "Homebrew tap failed — falling back to source build (libmcrypt 2.5.8)"

    local dir="$TMPDIR_BUILD/libmcrypt"
    mkdir -p "$dir"
    # Try direct SF mirror first; fall back to redirect URL; both with a hard timeout
    curl -fsSL --max-time 120 \
        "https://downloads.sourceforge.net/project/mcrypt/Libmcrypt/2.5.8/libmcrypt-2.5.8.tar.bz2" \
        -o "$dir/libmcrypt.tar.bz2" || \
    curl -fsSL --max-time 120 \
        "https://sourceforge.net/projects/mcrypt/files/Libmcrypt/2.5.8/libmcrypt-2.5.8.tar.bz2/download" \
        -o "$dir/libmcrypt.tar.bz2"
    tar xjf "$dir/libmcrypt.tar.bz2" -C "$dir" --strip-components=1
    (
        cd "$dir"
        # config.sub from 2001 doesn't recognise aarch64-apple-darwin; refresh it
        curl -fsSL "https://git.savannah.gnu.org/cgit/config.git/plain/config.sub" -o config.sub
        curl -fsSL "https://git.savannah.gnu.org/cgit/config.git/plain/config.guess" -o config.guess
        chmod +x config.sub config.guess
        # Suppress C89-era implicit declaration/int errors — libmcrypt 2.5.8 predates C99
        CFLAGS="-Wno-implicit-function-declaration -Wno-implicit-int -Wno-int-conversion -Wno-pointer-sign" \
        ./configure --prefix="$PREFIX" --disable-posix-threads --enable-static
        make -j"$(ncpu)"
        $(maybe_sudo) make install
    )
    log "libmcrypt installed"
}

# ── Install libmhash (not in Homebrew core) ───────────────────────────────────
build_mhash() {
    if pkg-config --exists mhash 2>/dev/null; then
        log "libmhash already installed"
        return
    fi
    for ext in so dylib a; do
        [ -f "${PREFIX}/lib/libmhash.$ext" ] && { log "libmhash already installed"; return; }
    done

    # Try the discoteq tap first
    log "Installing libmhash via Homebrew tap (discoteq/discoteq)..."
    if brew list discoteq/discoteq/libmhash &>/dev/null || brew install discoteq/discoteq/libmhash &>/dev/null; then
        log "libmhash installed via Homebrew"
        return
    fi
    warn "Homebrew tap failed — falling back to source build (mhash 0.9.9.9)"

    local dir="$TMPDIR_BUILD/mhash"
    mkdir -p "$dir"
    curl -fsSL --max-time 120 \
        "https://downloads.sourceforge.net/project/mhash/mhash/0.9.9.9/mhash-0.9.9.9.tar.bz2" \
        -o "$dir/mhash.tar.bz2" || \
    curl -fsSL --max-time 120 \
        "https://sourceforge.net/projects/mhash/files/mhash/0.9.9.9/mhash-0.9.9.9.tar.bz2/download" \
        -o "$dir/mhash.tar.bz2"
    tar xjf "$dir/mhash.tar.bz2" -C "$dir" --strip-components=1
    (
        cd "$dir"
        # Refresh config.sub/config.guess for modern architectures (aarch64, etc.)
        curl -fsSL "https://git.savannah.gnu.org/cgit/config.git/plain/config.sub" -o config.sub
        curl -fsSL "https://git.savannah.gnu.org/cgit/config.git/plain/config.guess" -o config.guess
        chmod +x config.sub config.guess
        CFLAGS="-Wno-implicit-function-declaration -Wno-implicit-int -Wno-int-conversion -Wno-pointer-sign" \
        ./configure --prefix="$PREFIX"
        make -j"$(ncpu)"
        $(maybe_sudo) make install
    )
    log "libmhash installed"
}

# ── Build libgringotts from source ────────────────────────────────────────────
build_libgringotts() {
    local extra_flags="${1:-}"
    log "Cloning libgringotts..."
    need git
    local dir="$TMPDIR_BUILD/libgringotts"
    git clone --depth=1 https://gitlab.com/deb-pkg/libgringotts.git "$dir"
    (
        cd "$dir"
        autoreconf -fi
        # shellcheck disable=SC2086
        eval "./configure --prefix='$PREFIX' $extra_flags"
        make -j"$(ncpu)"
        $(maybe_sudo) make install
    )
    log "libgringotts installed to $PREFIX"
}

# ── macOS ─────────────────────────────────────────────────────────────────────
install_macos() {
    need brew
    xcode-select -p &>/dev/null || die "Xcode Command Line Tools missing — run: xcode-select --install"

    log "Installing build tools via Homebrew..."
    # Install each tool separately so a missing formula doesn't abort the rest.
    # libmcrypt and libmhash are not in Homebrew core — we build them from source below.
    for pkg in autoconf automake libtool pkg-config; do
        brew list "$pkg" &>/dev/null || brew install "$pkg"
    done

    local BREW_PREFIX
    BREW_PREFIX="$(brew --prefix)"

    build_libmcrypt
    build_mhash

    local extra
    extra="LDFLAGS='-L${BREW_PREFIX}/lib -L${PREFIX}/lib' \
           CPPFLAGS='-I${BREW_PREFIX}/include -I${PREFIX}/include' \
           PKG_CONFIG_PATH='${BREW_PREFIX}/lib/pkgconfig:${PREFIX}/lib/pkgconfig'"
    build_libgringotts "$extra"
}

# ── Linux ─────────────────────────────────────────────────────────────────────
install_linux() {
    if command -v apt-get &>/dev/null; then
        log "Debian/Ubuntu: installing via apt..."
        sudo apt-get update -q
        sudo apt-get install -y libgringotts-dev || {
            warn "libgringotts-dev not found in apt — building from source"
            install_linux_from_source
        }

    elif command -v dnf &>/dev/null; then
        log "Fedora/RHEL: installing via dnf..."
        sudo dnf install -y libgringotts-devel || {
            warn "libgringotts-devel not found — building from source"
            install_linux_from_source
        }

    elif command -v zypper &>/dev/null; then
        log "openSUSE: installing via zypper..."
        sudo zypper install -y libgringotts-devel || {
            warn "Package not found — building from source"
            install_linux_from_source
        }

    elif command -v pacman &>/dev/null; then
        # Arch: try AUR helper, otherwise build from source
        if command -v yay &>/dev/null; then
            yay -S --noconfirm libgringotts || install_linux_from_source
        else
            install_linux_from_source
        fi

    else
        warn "Unknown package manager — building from source"
        install_linux_from_source
    fi
}

install_linux_from_source() {
    need git
    need make
    need gcc

    # Install build dependencies if apt is available
    if command -v apt-get &>/dev/null; then
        sudo apt-get install -y \
            autoconf automake libtool pkg-config \
            libmcrypt-dev libmhash-dev zlib1g-dev
        # If libmcrypt-dev / libmhash-dev are present, skip building them
    else
        build_libmcrypt
        build_mhash
    fi

    build_libgringotts
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
    log "Checking for libgringotts..."

    if already_installed; then
        log "Nothing to do."
        exit 0
    fi

    # Warm up sudo credentials now so 'make install' doesn't stall interactively later
    if [[ "$PREFIX" == /usr* ]]; then
        log "Installing to $PREFIX — sudo required. Enter your password now:"
        sudo -v || die "sudo authentication failed"
    fi

    OS="$(uname -s)"
    case "$OS" in
        Darwin) install_macos ;;
        Linux)  install_linux ;;
        *)
            die "Unsupported OS: $OS
Windows users: install WSL2 and run this script inside the WSL2 shell,
or use setup.ps1 from an MSYS2 MinGW64 shell."
            ;;
    esac

    log ""
    log "libgringotts is ready at: $PREFIX"
    if [[ "$PREFIX" != "/usr/local" && "$PREFIX" != "/usr" ]]; then
        log ""
        log "Non-standard prefix detected. Before running 'cargo build', export:"
        log "  export LIBGRINGOTTS_DIR=\"$PREFIX\""
        log "  export LIBRARY_PATH=\"$PREFIX/lib:\$LIBRARY_PATH\""
        log "  export PKG_CONFIG_PATH=\"$PREFIX/lib/pkgconfig:\${PKG_CONFIG_PATH:-}\""
    fi
}

main "$@"
