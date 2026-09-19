#!/bin/sh
# Download a complete release: CLI, daemon, pinned TDLib and embedded agent skills.

# Parse the complete function before doing any work when piped from curl.
main() {
    set -eu

    version=latest
    prefix=${TELEGRAM_INSTALL_PREFIX:-"$HOME/.local"}
    skill=both
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version|--prefix|--skill)
                [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
                case "$1" in --version) version=$2;; --prefix) prefix=$2;; --skill) skill=$2;; esac
                shift 2;;
            --help|-h)
                echo 'Usage: install-release.sh [--version TAG] [--prefix DIR] [--skill codex|claude|both|none]'
                echo 'Downloads binaries and TDLib; no Rust, Python or local build. Default: latest release, ~/.local, both skills.'
                exit 0;;
            *) echo "unknown option: $1" >&2; exit 2;;
        esac
    done
    case "$version" in ''|[!a-zA-Z0-9]*|*[!a-zA-Z0-9._-]*) echo 'invalid release tag' >&2; exit 2;; esac
    case "$prefix" in /*) ;; *) echo '--prefix must be absolute' >&2; exit 2;; esac
    case "$skill" in codex|claude|both|none) ;; *) echo 'invalid --skill' >&2; exit 2;; esac
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64) target=aarch64-apple-darwin;;
        Darwin-x86_64) echo 'On Apple Silicon, use a native arm64 terminal (not Rosetta). Intel Macs are unsupported.' >&2; exit 2;;
        Linux-x86_64) target=x86_64-unknown-linux-gnu;;
        *) echo 'Supported: macOS arm64 and Linux x86_64 GNU/glibc' >&2; exit 2;;
    esac
    for command in curl tar; do
        command -v "$command" >/dev/null 2>&1 || { echo "$command is required" >&2; exit 2; }
    done
    if command -v sha256sum >/dev/null 2>&1; then hash=sha256sum
    elif command -v shasum >/dev/null 2>&1; then hash=shasum
    else echo 'sha256sum or shasum is required' >&2; exit 2
    fi

    releases=https://github.com/lonmstalker/telegram-cli/releases
    base=$releases
    if [ "$version" = latest ]; then base="$base/latest/download"
    else base="$base/download/$version"
    fi
    asset="telegram-cli-$target.tar.gz"
    stage=$(mktemp -d)
    trap 'rm -rf "$stage"' EXIT
    trap 'exit 130' HUP INT TERM
    for file in "$asset" "$asset.sha256"; do
        if ! curl --fail --location --silent --show-error --proto '=https' --proto-redir '=https' \
            --connect-timeout 15 --max-time 300 "$base/$file" -o "$stage/$file"; then
            echo "Cannot download $file for $version. See curl's error above and platform assets: $releases" >&2
            exit 1
        fi
    done
    read -r expected filename extra < "$stage/$asset.sha256" || [ -n "$expected" ] || {
        echo 'Empty checksum file' >&2; exit 1;
    }
    case "$expected" in ''|*[!a-fA-F0-9]*) echo 'Invalid SHA-256 checksum' >&2; exit 1;; esac
    [ "${#expected}" -eq 64 ] && [ "$filename" = "$asset" ] && [ -z "$extra" ] || {
        echo 'Invalid checksum file' >&2; exit 1;
    }
    if [ "$hash" = sha256sum ]; then actual=$(sha256sum "$stage/$asset")
    else actual=$(shasum -a 256 "$stage/$asset")
    fi
    [ "${actual%% *}" = "$expected" ] || { echo 'SHA-256 mismatch; installation cancelled' >&2; exit 1; }

    # Reject paths outside the bundle before extraction. Assets come from this repository over HTTPS.
    tar -tzf "$stage/$asset" > "$stage/members"
    while IFS= read -r member; do
        case "/$member/" in */../*|*/./*) echo 'Unsafe bundle path' >&2; exit 1;; esac
        case "$member" in telegram-cli|telegram-cli/*) ;; *) echo 'Invalid bundle layout' >&2; exit 1;; esac
    done < "$stage/members"
    tar -xzf "$stage/$asset" -C "$stage"
    sh "$stage/telegram-cli/install.sh" --prefix "$prefix" --skill "$skill"
}

main "$@"
