#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

declare VER=${1:-"linux-rolling-stable"}

# nix-prefetch -v fetchFromGitHub --owner "gregkh" --repo "linux" --rev "${VER}" 2>&1

echo -e "Downloading Linux Kernel ${VER} from archive..."

nix-prefetch-url --unpack https://github.com/gregkh/linux/archive/${VER}.tar.gz | \
{ HASH="$(</dev/stdin)"; nix-hash --type sha256 --to-sri $HASH 2> /dev/null; \
nix-hash --type sha256 --to-base64 $HASH 2> /dev/null; } | \
{ NIX_HASH="$(</dev/stdin)"; \
echo -e "Nix-Hash for Linux Kernel ${VER}.tar.gz == $(echo $NIX_HASH | awk 'NR==1{print $2}') and full nix-hash == $(echo $NIX_HASH | awk 'NR==1{print $1}')"; }
