#!/bin/bash
# Cross-compile SILE v0.15.13 for Windows from Linux. The reproducible form of
# spike/S1-NOTES.md P-9 — every flag here is a failure that was hit and fixed,
# in the order the build hits them.
#
# Run inside fedora:41 (which is the only distro checked that packages the whole
# mingw-w64 stack SILE needs):
#
#     docker run --rm -v "$PWD:/out" fedora:41 bash /out/s1-windows-cross.sh
#
# Produces /work/stage — sile.exe plus everything it needs beside it.
#
# Fedora's mingw64-icu carries NO break-iterator data (P-9), so ICU is taken
# from MSYS2 instead — along with MSYS2's GCC runtime, because ICU is C++
# internally and the two distributions' libstdc++ do not interchange (P-10).
set -euo pipefail

SILE_REF=${SILE_REF:-v0.15.13}
SYS=/usr/x86_64-w64-mingw32/sys-root/mingw
WORK=${WORK:-/work}
mkdir -p "$WORK"

say () { printf '\n=== %s ===\n' "$1"; }

say "prerequisites"
# mingw64-*: the cross stack. HarfBuzz here is 9.0.0, newer than the Ubuntu
#   native build had, so font variations and subsetting are available.
# diffutils/poppler-utils: configure hard-requires cmp and pdfinfo.
# luajit/luarocks: configure must RUN a 5.1 interpreter; cross-compiling cannot
#   run the target one, so a native one stands in for the version probes while
#   LUA_INCLUDE/LUA_LIB point at the sysroot.
# rust-std-static-x86_64-pc-windows-gnu: the cross target, without rustup.
dnf -y install \
  mingw64-gcc mingw64-gcc-c++ mingw64-harfbuzz mingw64-icu mingw64-fontconfig \
  mingw64-freetype mingw64-libpng mingw64-zlib mingw64-expat \
  mingw64-filesystem mingw64-binutils mingw64-winpthreads-static \
  autoconf automake libtool pkgconf-pkg-config make gcc gcc-c++ jq git \
  cargo rust rust-std-static-x86_64-pc-windows-gnu \
  diffutils poppler-utils luajit luajit-devel luarocks \
  file which findutils patch python3 >/dev/null
ln -sf /usr/bin/luajit /usr/bin/lua-5.1   # luarocks looks for this name

say "cross-build LuaJIT (there is no mingw64-luajit package)"
cd "$WORK"
[ -d LuaJIT ] || git clone -q --depth 1 -b v2.1 https://github.com/LuaJIT/LuaJIT.git
cd LuaJIT/src
make -C .. -j"$(nproc)" HOST_CC="gcc -m64" CROSS=x86_64-w64-mingw32- \
     TARGET_SYS=Windows BUILDMODE=dynamic >/dev/null
mkdir -p "$SYS/include/luajit-2.1" "$SYS/bin" "$SYS/lib/pkgconfig"
cp lua.h luaconf.h lualib.h lauxlib.h luajit.h "$SYS/include/luajit-2.1/"
cp lua51.dll luajit.exe "$SYS/bin/"
cp libluajit-5.1.dll.a "$SYS/lib/"
cat > "$SYS/lib/pkgconfig/luajit.pc" <<EOF
prefix=$SYS
libdir=\${prefix}/lib
includedir=\${prefix}/include/luajit-2.1
Name: LuaJIT
Description: Just-in-time compiler for Lua
Version: 2.1.0-beta3
Libs: -L\${libdir} -lluajit-5.1
Cflags: -I\${includedir}
EOF

say "ICU and the C++ runtime from MSYS2, not Fedora"
# Fedora's mingw64-icu is data-filtered: zero brkitr resources, so SILE cannot
# break lines (U_MISSING_RESOURCE_ERROR). MSYS2's has all 36 plus the five
# dictionaries for scripts written without spaces. MSYS2 packages are plain
# zstd tarballs; no MSYS2 installation is needed to use one.
cd "$WORK"
fetch_msys2 () {  # $1 = package name glob, extracts into $2
  local p
  p=$(curl -s https://repo.msys2.org/mingw/mingw64/ \
      | grep -oE "$1-[0-9][^\"]*\.pkg\.tar\.zst" | sort -V | tail -1)
  [ -f "$p" ] || curl -sLO "https://repo.msys2.org/mingw/mingw64/$p"
  mkdir -p "$2" && tar --use-compress-program=unzstd -xf "$p" -C "$2"
  echo "  $p"
}
fetch_msys2 mingw-w64-x86_64-icu       "$WORK/msys2icu"
fetch_msys2 mingw-w64-x86_64-gcc-libs  "$WORK/msys2gcc"
fetch_msys2 mingw-w64-x86_64-libwinpthread-git "$WORK/msys2gcc"

# Retire Fedora's ICU so pkg-config cannot find it, then install MSYS2's.
mkdir -p "$WORK/icu-retired"
for f in "$SYS"/bin/icu*.dll "$SYS"/lib/libicu*.dll.a "$SYS"/lib/pkgconfig/icu-*.pc; do
  [ -e "$f" ] && mv "$f" "$WORK/icu-retired/" || true
done
rm -rf "$SYS/include/unicode"
cp "$WORK"/msys2icu/mingw64/bin/libicu*.dll "$SYS/bin/"
cp "$WORK"/msys2icu/mingw64/lib/libicu*.dll.a "$SYS/lib/"
cp -r "$WORK"/msys2icu/mingw64/include/unicode "$SYS/include/"
for pc in "$WORK"/msys2icu/mingw64/lib/pkgconfig/icu-*.pc; do
  sed -e "s|^prefix=.*|prefix=$SYS|" -e "s|/mingw64|$SYS|g" "$pc" \
    > "$SYS/lib/pkgconfig/$(basename "$pc")"
done

say "source"
cd "$WORK"
[ -d sile-src ] || git clone -q --depth 1 --branch "$SILE_REF" --recurse-submodules \
  https://github.com/sile-typesetter/sile.git sile-src
cd sile-src

say "patch two portability bugs (4 lines)"
# libtexpdf's MinGW branch is stale: modern mingw-w64 declares ftello64/fseeko64
# itself with _off64_t, conflicting with the _off_t prototypes these rewrite,
# and provides ftello/fseeko directly.
python3 - <<'PY'
import io
p = "libtexpdf/libtexpdf.h"
s = open(p).read()
old = "#elif defined(__MINGW32__)\n#define ftello ftello64\n#define fseeko fseeko64\n#endif"
if old in s:
    open(p, "w").write(s.replace(old, "#elif defined(__MINGW32__)\n/* stale; mingw-w64 provides these */\n#endif"))
# silewin32.h implements strcasestr() with tolower() and never includes ctype.h.
p = "justenough/silewin32.h"
s = open(p).read()
if not s.startswith("#include <ctype.h>"):
    open(p, "w").write("#include <ctype.h>\n" + s)
PY

say "bootstrap"
[ -f configure ] || ./bootstrap.sh >/dev/null 2>&1

say "configure for x86_64-w64-mingw32"
export PKG_CONFIG_PATH=$SYS/lib/pkgconfig PKG_CONFIG_LIBDIR=$SYS/lib/pkgconfig
# The Rust half asks pkg-config separately, through the `pkg-config` crate,
# which REFUSES to answer while cross-compiling unless told this — on the
# grounds that a build script reading the host's libraries is usually a
# mistake. Here it is not: PKG_CONFIG_LIBDIR above points at the mingw sysroot
# and nothing else, so what it finds is the target's LuaJIT.
#
# Without it `mlua-sys` fails with "cannot find LuaJIT using pkg-config:
# pkg-config has not been configured to support cross-compilation", which
# names neither the variable nor the reason.
export PKG_CONFIG_ALLOW_CROSS=1
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
# --program-prefix=          autoconf otherwise prefixes ${host_alias}- and the
#                            generated rockspec's package name stops matching
#                            its filename, which luarocks rejects.
# --with-system-lua-sources  MANDATORY. Without it mlua links LuaJIT statically
#                            into sile.exe, which exports nothing, while the
#                            rock DLLs bind to lua51.dll — two VMs in one
#                            process. Fails far away as "attempt to perform
#                            arithmetic on a boolean value".
# CARGO_TARGET_TRIPLE        defaults to the BUILD machine's rustc host.
mingw64-configure \
  --disable-dependency-tracking \
  --program-prefix= \
  --with-system-lua-sources \
  CARGO_TARGET_TRIPLE=x86_64-pc-windows-gnu \
  LUA_INCLUDE="-I$SYS/include/luajit-2.1" \
  LUA_LIB="-L$SYS/lib -lluajit-5.1" \
  FCMATCH=true >/dev/null

# configure GENERATES aminclude.am, leaving it newer than Makefile.in, so make
# re-runs automake — which then rejects the file configure just wrote
# ('rusile.dll' is not a standard library name). Stop the regeneration.
touch aclocal.m4 configure aminclude.am
find . -name 'Makefile.in' -exec touch {} +
touch Makefile

say "make"
T=target/x86_64-pc-windows-gnu/release
make -j"$(nproc)" >/dev/null 2>&1 || true
# Windows DLLs take no lib prefix and binaries end in .exe; the Makefile
# hardcodes Unix names for both.
[ -f "$T/rusile.dll" ] && cp -f "$T/rusile.dll" "$T/librusile.dll"
make -j"$(nproc)" >/dev/null 2>&1 || true
[ -f "$T/sile.exe" ] && cp -f "$T/sile.exe" "$T/sile"
make -j"$(nproc)" >/dev/null
test -f "$T/sile.exe"
echo "  sile.exe: $(du -h "$T/sile.exe" | cut -f1)"

say "cross-build the C rocks"
# luarocks emits `gcc -shared -L… -lluajit-5.1 -o out.so obj.o`, library before
# object; GNU ld discards it before anything needs it. Append it instead.
cat > /usr/local/bin/xgcc-lua <<EOF
#!/bin/sh
exec x86_64-w64-mingw32-gcc "\$@" -L$SYS/lib -lluajit-5.1
EOF
chmod +x /usr/local/bin/xgcc-lua

XR=$WORK/xrocks
rm -rf "$XR"
common=(CC=x86_64-w64-mingw32-gcc LD=xgcc-lua AR=x86_64-w64-mingw32-ar
        RANLIB=x86_64-w64-mingw32-ranlib LUA_INCDIR="$SYS/include/luajit-2.1"
        LUA_LIBDIR="$SYS/lib" CFLAGS="-O2 -Wall -fPIC" LIBFLAG="-shared")
# bit32 (POSIX strerror_r) and linenoise (termios.h) are omitted: bit32 is not
# needed under LuaJIT, and linenoise is the REPL line editor.
for r in luautf8 lpeg luafilesystem compat53; do
  luarocks --tree "$XR" --lua-version 5.1 install "$r" "${common[@]}" >/dev/null
done
luarocks --tree "$XR" --lua-version 5.1 install luaexpat "${common[@]}" \
  EXPAT_INCDIR="$SYS/include" EXPAT_LIBDIR="$SYS/lib" >/dev/null
luarocks --tree "$XR" --lua-version 5.1 install lua-zlib "${common[@]}" \
  ZLIB_INCDIR="$SYS/include" ZLIB_LIBDIR="$SYS/lib" >/dev/null
echo "  $(find "$XR" -name '*.so' | wc -l) modules, all PE32+"

say "stage"
ST=$WORK/stage
rm -rf "$ST"; mkdir -p "$ST/lua_modules/lib/lua/5.1" "$ST/lua_modules/share/lua/5.1"
cp "$T/sile.exe" "$T/rusile.dll" "$ST/"
cp justenough/.libs/*.dll libtexpdf/.libs/libtexpdf-0.dll "$ST/"
# SILE's own Lua. lua-libraries is flattened because SILE looks for it under the
# compiled-in $libdir/sile, which does not exist on Windows.
cp -r core classes packages languages types typesetters shapers outputters \
      inputters pagebuilders "$ST/"
cp -r lua-libraries/. "$ST/"
cp -r lua_modules/share/lua/5.1/. "$ST/lua_modules/share/lua/5.1/"
cp -r "$XR"/lib*/lua/5.1/. "$ST/lua_modules/lib/lua/5.1/"
[ -d "$XR/share/lua/5.1" ] && cp -r "$XR/share/lua/5.1/." "$ST/lua_modules/share/lua/5.1/"

# The pure-Lua rocks the cross-build does not leave behind.
#
# `make` populates lua_modules for the BUILD machine, and nine of the twelve
# arrive; vstruct, lua_cliargs and luarepl do not. SILE opens vstruct the
# moment it reads a font, so a stage without it cannot set a single page —
# and says so as `module 'vstruct' not found`, on the target machine, with a
# list of paths that all look plausible.
#
# Installed with the host's luarocks and no cross-compilation, because there
# is nothing to compile: these are Lua source, and a `.lua` file is the same
# file on every architecture.
say "the pure-Lua rocks make leaves out"
for rock in vstruct lua_cliargs luarepl; do
  luarocks --lua-version 5.1 install --tree "$ST/lua_modules" --deps-mode none     "$rock" >/dev/null 2>&1 || echo "  could not install $rock"
done
printf '  lua modules staged: %s
'   "$(find "$ST/lua_modules/share/lua/5.1" -maxdepth 1 -mindepth 1 -printf '%f ' 2>/dev/null)"

# Transitive DLL closure, walked rather than guessed — Fedora names ICU
# icuuc74.dll with no lib prefix, which hand-written lists miss.
OD=x86_64-w64-mingw32-objdump
changed=1
while [ $changed -eq 1 ]; do
  changed=0
  while IFS= read -r f; do
    while IFS= read -r d; do
      [ -f "$ST/$d" ] && continue
      [ -f "$SYS/bin/$d" ] && cp "$SYS/bin/$d" "$ST/" && changed=1
    done < <($OD -p "$f" 2>/dev/null | sed -n 's/^[[:space:]]*DLL Name:[[:space:]]*//p')
  done < <(find "$ST" -name '*.dll' -o -name '*.exe' -o -name '*.so')
done

# ICU is C++ internally and MSYS2 built it with a newer GCC than Fedora's, so
# the C++ runtime must come from MSYS2 as well. Skipping this fails at load with
# "The specified procedure could not be found", naming neither library nor cause.
cp "$WORK"/msys2gcc/mingw64/bin/*.dll "$ST/"

printf '  %s files, %s, %s DLLs\n' \
  "$(find "$ST" -type f | wc -l)" "$(du -sh "$ST" | cut -f1)" \
  "$(find "$ST" -maxdepth 1 -name '*.dll' | wc -l)"
echo
echo "Copy $ST to a Windows machine and run sile.exe from inside it."
echo "A fonts.conf naming C:/Windows/Fonts is needed via FONTCONFIG_FILE."
