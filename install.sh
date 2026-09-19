#!/bin/sh
# Local source checkout or an unpacked release bundle. No sudo, shell edits or native build.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${TELEGRAM_INSTALL_PREFIX:-"$HOME/.local"}
native=
skill=none
build=yes
while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix|--native|--skill)
            [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
            case "$1" in --prefix) prefix=$2;; --native) native=$2;; --skill) skill=$2;; esac
            shift 2;;
        --no-build) build=no; shift;;
        --help|-h)
            echo 'Usage: ./install.sh [--prefix DIR] [--native PATH] [--skill codex|claude|both|none] [--no-build]'
            echo 'Default prefix: ~/.local. Source install needs Rust and Python 3; native TDLib is never built implicitly.'
            exit 0;;
        *) echo "unknown option: $1" >&2; exit 2;;
    esac
done
case "$prefix" in /*) ;; *) echo '--prefix must be absolute' >&2; exit 2;; esac
case "$skill" in codex|claude|both|none) ;; *) echo 'invalid --skill' >&2; exit 2;; esac
case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) target=aarch64-apple-darwin; library=libtdjson.dylib;;
    Darwin-x86_64) echo 'On Apple Silicon, use a native arm64 terminal (not Rosetta). Intel Macs are unsupported.' >&2; exit 2;;
    Linux-x86_64) target=x86_64-unknown-linux-gnu; library=libtdjson.so;;
    *) echo 'Supported: macOS arm64 and Linux x86_64' >&2; exit 2;;
esac

if [ -f "$root/Cargo.toml" ]; then
    binaries="$root/target/release"
    if [ -z "$native" ]; then
        command -v python3 >/dev/null 2>&1 || { echo 'Python 3 is needed to read the pinned artifact manifest' >&2; exit 2; }
        native=$(python3 - "$root" "$target" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / 'vendor/tdlib/native-builds' / (sys.argv[2] + '.json')).read_text())
print(root / manifest['artifact']['cache_path'])
PY
)
    fi
    [ -f "$native" ] || { echo 'Pinned TDLib artifact missing. Supply --native /absolute/path/to/libtdjson or use a release bundle; see README.md.' >&2; exit 2; }
    if [ "$build" = yes ]; then
        command -v cargo >/dev/null 2>&1 || { echo 'Rust/Cargo is required for a source install' >&2; exit 2; }
        (cd "$root" && cargo build --locked --release -q -j 2 -p telegram-cli -p telegramd)
    fi
else
    binaries="$root/bin"
    [ -n "$native" ] || native="$root/lib/$library"
fi
[ -x "$binaries/telegram-cli" ] && [ -x "$binaries/telegramd" ] || { echo 'CLI/daemon binaries missing; build first' >&2; exit 2; }
case "$native" in /*) ;; *) native="$(pwd)/$native";; esac
"$binaries/telegramd" --check-native "$native"
"$binaries/telegram-cli" --version

mkdir -p "$prefix/bin" "$prefix/lib/telegram-cli"
stage=$(mktemp -d "$prefix/.telegram-cli-install.XXXXXX")
trap 'rm -rf "$stage"' EXIT
trap 'exit 130' HUP INT TERM
install -m 755 "$binaries/telegram-cli" "$stage/telegram-cli"
install -m 755 "$binaries/telegramd" "$stage/telegramd"
install -m 644 "$native" "$stage/$library"
mv -f "$stage/$library" "$prefix/lib/telegram-cli/$library"
mv -f "$stage/telegramd" "$prefix/bin/telegramd"
mv -f "$stage/telegram-cli" "$prefix/bin/telegram-cli"
case "$skill" in codex|both) "$prefix/bin/telegram-cli" init --global;; esac
case "$skill" in claude|both) "$prefix/bin/telegram-cli" init --global --claude;; esac
echo "Installed to $prefix/bin. Add that directory to PATH if needed."
echo 'Next: telegram-cli setup (once, in your own terminal), then telegram-cli --agent login'
