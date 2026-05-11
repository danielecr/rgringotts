#!/usr/bin/env bash
# docker-build.sh — build rgringotts for Linux inside Docker
#
# Produces:
#   dist/rgringotts          — Linux amd64 binary
#   docker image 'rgringotts' — runnable container
#
# Usage:
#   ./docker-build.sh              # build image + extract binary
#   ./docker-build.sh --image-only # build image only, no extraction
set -euo pipefail

IMAGE="rgringotts"
OUTDIR="$(dirname "$0")/dist"

log()  { printf '  \033[32m[docker-build]\033[0m %s\n' "$*"; }
die()  { printf '  \033[31m[docker-build] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

command -v docker &>/dev/null || die "docker not found — install Docker Desktop first"

log "Building Docker image '$IMAGE' ..."
docker build -t "$IMAGE" "$(dirname "$0")"

if [[ "${1:-}" == "--image-only" ]]; then
    log "Image built. Run with: docker run -p 7979:7979 $IMAGE"
    exit 0
fi

# Extract the binary from the image
mkdir -p "$OUTDIR"
CID=$(docker create "$IMAGE")
docker cp "$CID:/usr/local/bin/rgringotts" "$OUTDIR/rgringotts"
docker rm "$CID" >/dev/null

log "Binary extracted to: $OUTDIR/rgringotts"
log "Run on a Linux server: scp $OUTDIR/rgringotts user@host:/usr/local/bin/"
log "Or run locally in Docker: docker run -p 7979:7979 $IMAGE"
