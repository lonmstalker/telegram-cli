#!/bin/sh
# Create a portable local bundle from the same verified installation layout.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output=${1:?Usage: scripts/package-release.sh /absolute/output.tar.gz [native-library]}
case "$output" in /*.tar.gz) ;; *) echo 'Output must be an absolute .tar.gz path' >&2; exit 2;; esac
[ ! -e "$output" ] && [ ! -e "$output.sha256" ] || { echo 'Output already exists' >&2; exit 2; }
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
trap 'exit 130' HUP INT TERM
if [ "$#" -gt 1 ]; then
    "$root/install.sh" --no-build --prefix "$stage/telegram-cli" --native "$2"
else
    "$root/install.sh" --no-build --prefix "$stage/telegram-cli"
fi
mv "$stage/telegram-cli/lib/telegram-cli/"* "$stage/telegram-cli/lib/"
rmdir "$stage/telegram-cli/lib/telegram-cli"
cp "$root/install.sh" "$stage/telegram-cli/"
cat > "$stage/telegram-cli/README.md" <<'README'
# Telegram CLI bundle

macOS arm64 / Linux x86_64 GNU bundles include CLI, daemon and the pinned TDLib library.
No Rust toolchain or native build is needed to install this bundle:

```sh
./install.sh --skill both
# Add ~/.local/bin to PATH if needed.
telegram-cli setup
telegram-cli --agent login
telegram-cli --agent workflow list
telegram-cli --help
```

Run setup in the owner's terminal. The saved profile is reused by agents; do not
send API credentials, phone, OTP, 2FA or database keys to an agent. Default scopes
are read-only. Before upgrading, let the existing daemon finish and reach idle Closed.
Profiles and keys are outside the installation prefix and are not replaced by installation.

Full source and documentation: https://github.com/lonmstalker/telegram-cli
README
case "$(uname -s)" in
    Darwin) COPYFILE_DISABLE=1 tar --uid 0 --gid 0 --uname root --gname root -czf "$output" -C "$stage" telegram-cli;;
    *) tar --owner=0 --group=0 --numeric-owner -czf "$output" -C "$stage" telegram-cli;;
esac
(
    cd "$(dirname -- "$output")"
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$(basename -- "$output")"
    else shasum -a 256 "$(basename -- "$output")"
    fi
) > "$output.sha256"
echo "Bundle: $output"
echo "Checksum: $output.sha256"
