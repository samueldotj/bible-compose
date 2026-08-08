#!/bin/bash
# S0 toolchain — SILE 0.15.13 on Ubuntu 22.04 (x86_64).
#
# The upstream binary is not self-contained: it embeds LuaJIT and SILE's own
# Lua, but none of its 20 third-party Lua rocks. See NOTES.md F-2.
#
# Run the apt step once, with sudo. Everything after it is user-local:
# nothing is written outside $HOME.
#
#   sudo ./setup-wsl.sh --system     # apt packages only
#   ./setup-wsl.sh                   # binary + rocks, no root
#
set -euo pipefail

SILE_VERSION=0.15.13
SILE_SHA256=f9f875447ecade9515e984ee66039c67d64c99fbfc904f95fe7f1ed0edbfe194
PREFIX="$HOME/.local"
ROCKS="$HOME/.luarocks"

# --- system packages ---------------------------------------------------------
# lua5.1 + headers: luarocks needs them to build the C rocks against the 5.1
#   ABI, which is what the binary's embedded LuaJIT 2.1 speaks.
# zlib1g-dev, libssl-dev: required by lua-zlib and luasec.
# poppler-utils: pdfinfo and pdftoppm, for measuring and rendering output.
if [ "${1:-}" = "--system" ]; then
  apt-get update
  apt-get install -y --no-install-recommends \
    lua5.1 liblua5.1-0-dev luarocks \
    libexpat1-dev zlib1g-dev libssl-dev \
    poppler-utils
  echo "system packages installed; now re-run without --system as your user"
  exit 0
fi

command -v luarocks >/dev/null || {
  echo "luarocks missing — run: sudo $0 --system" >&2
  exit 1
}

# --- SILE binary -------------------------------------------------------------
mkdir -p "$PREFIX/bin"
if [ ! -x "$PREFIX/bin/sile" ]; then
  tmp=$(mktemp -d)
  curl -fsSL --retry 3 -o "$tmp/sile" \
    "https://github.com/sile-typesetter/sile/releases/download/v$SILE_VERSION/sile-x86_64"
  echo "$SILE_SHA256  $tmp/sile" | sha256sum -c -
  install -m 0755 "$tmp/sile" "$PREFIX/bin/sile"
  rm -rf "$tmp"
fi

# --- Lua rocks ---------------------------------------------------------------
# Pinned to the versions in sile.rockspec.in for v0.15.13. Installed --local so
# no root is needed and so a bad set can be discarded with rm -rf ~/.luarocks.
rocks=(
  "bit32"                "cassowary 2.3.2-1"    "cldr 0.3.0-0"
  "compat53 0.14.4-1"    "fluent 0.2.0-0"       "linenoise 0.9-1"
  "loadkit 1.1.0-1"      "lpeg 1.1.0-2"         "lua-zlib 1.3-0"
  "lua_cliargs 3.0.2-1"  "luaepnf 0.3-2"        "luaexpat 1.5.2-1"
  "luafilesystem 1.8.0-1" "luarepl 0.10-1"      "luasec 1.3.2-1"
  "luasocket 3.1.0-1"    "luautf8 0.1.6-1"      "penlight 1.14.0-3"
  "vstruct 2.1.1-1"
)
for r in "${rocks[@]}"; do
  # shellcheck disable=SC2086
  luarocks --local --lua-version=5.1 install $r
done

# --- environment -------------------------------------------------------------
cat >"$PREFIX/bin/sile-env" <<'ENV'
# source this before running sile
export PATH="$HOME/.local/bin:$PATH"
eval "$(luarocks --local --lua-version=5.1 path)"
ENV

echo
echo "installed. to use:"
echo "  source ~/.local/bin/sile-env && sile --version"
