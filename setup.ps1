# setup.ps1 — install libgringotts on Windows before building rgringotts
#
# Preferred: run inside an MSYS2 MinGW64 shell (see https://www.msys2.org/)
#   bash setup.sh
#
# Alternatively this script uses WSL2 if available, or guides you to MSYS2.
#
# Run with:
#   Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
#   .\setup.ps1

$ErrorActionPreference = "Stop"

function Write-Step { param($msg) Write-Host "  [setup] $msg" -ForegroundColor Green }
function Write-Warn  { param($msg) Write-Host "  [setup] $msg" -ForegroundColor Yellow }
function Write-Fail  { param($msg) Write-Host "  [setup] ERROR: $msg" -ForegroundColor Red; exit 1 }

Write-Step "Checking environment..."

# ── Option 1: WSL2 ────────────────────────────────────────────────────────────
$wsl = Get-Command wsl -ErrorAction SilentlyContinue
if ($wsl) {
    Write-Step "WSL2 found — running setup.sh inside WSL..."
    $scriptDir = (Get-Item $PSScriptRoot).FullName
    # Convert Windows path to WSL path
    $wslPath = wsl wslpath -u "$scriptDir"
    wsl bash "$wslPath/setup.sh"
    Write-Step "Done via WSL2."
    exit 0
}

# ── Option 2: MSYS2 / pacman ──────────────────────────────────────────────────
$pacman = Get-Command pacman -ErrorAction SilentlyContinue
if ($pacman) {
    Write-Step "MSYS2 pacman found — installing dependencies..."
    # libgringotts is not in the MSYS2 repos; install build prerequisites
    & pacman -S --noconfirm --needed `
        mingw-w64-x86_64-gcc `
        mingw-w64-x86_64-autotools `
        mingw-w64-x86_64-pkg-config `
        mingw-w64-x86_64-libmcrypt `
        git make

    Write-Step "Building libmhash from source..."
    $tmp = Join-Path $env:TEMP "rgringotts-setup"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null

    $mhashDir = Join-Path $tmp "mhash"
    New-Item -ItemType Directory -Force -Path $mhashDir | Out-Null
    Invoke-WebRequest `
        "https://sourceforge.net/projects/mhash/files/mhash/0.9.9.9/mhash-0.9.9.9.tar.bz2/download" `
        -OutFile (Join-Path $mhashDir "mhash.tar.bz2")
    & tar xjf (Join-Path $mhashDir "mhash.tar.bz2") -C $mhashDir --strip-components=1
    Push-Location $mhashDir
    & bash -c "./configure && make -j4 && make install"
    Pop-Location

    Write-Step "Building libgringotts from source..."
    $grg = Join-Path $tmp "libgringotts"
    & git clone --depth=1 https://gitlab.com/deb-pkg/libgringotts.git $grg
    Push-Location $grg
    & bash -c "autoreconf -fi && ./configure && make -j4 && make install"
    Pop-Location

    Write-Step "libgringotts installed."
    exit 0
}

# ── Neither found ─────────────────────────────────────────────────────────────
Write-Warn ""
Write-Warn "Neither WSL2 nor MSYS2 was detected."
Write-Warn ""
Write-Warn "To build rgringotts on Windows, choose one of:"
Write-Warn ""
Write-Warn "  Option A — WSL2 (recommended):"
Write-Warn "    1. Enable WSL2:  wsl --install"
Write-Warn "    2. Open a WSL2 shell and run:  bash setup.sh"
Write-Warn "    3. Build inside WSL2:  cargo build"
Write-Warn ""
Write-Warn "  Option B — MSYS2 MinGW64:"
Write-Warn "    1. Install MSYS2 from https://www.msys2.org/"
Write-Warn "    2. Open the 'MSYS2 MinGW x64' shell."
Write-Warn "    3. Run:  bash setup.sh"
Write-Warn "    4. Install Rust for MSYS2: pacman -S mingw-w64-x86_64-rust"
Write-Warn "    5. Build:  cargo build"
Write-Warn ""
exit 1
