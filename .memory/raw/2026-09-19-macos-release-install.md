# macOS v0.1.0 public installation

- Date: 2026-09-19.
- Release: https://github.com/lonmstalker/telegram-cli/releases/tag/v0.1.0
- Source commit: `cf9050d0d09be0c9eae75313d1b22ad89da80184`.
- State verified using `gh release view`: published, not a draft/prerelease.
- Assets: `install-release.sh` (3785 bytes), `telegram-cli-aarch64-apple-darwin.tar.gz` (12323365 bytes), matching `.sha256` (107 bytes).
- Bundle SHA-256: `d8bd51fae04162088674ec9b80c0983469a622b8d2626da053b8373aec53be35`.
- `otool` check: CLI/daemon/TDLib minimum macOS 11.0; native dynamic dependencies are system zlib, libc++ and libSystem, no Homebrew paths.

## Network acceptance

An anonymous `/usr/bin/curl` downloaded the public latest `install-release.sh` asset;
its bytes matched the reviewed source. The script ran through `sh -s` in a temporary
HOME with PATH limited to system shell/download/archive/install/hash tools. Cargo,
Rust, Python, Git and Homebrew were unavailable through that PATH. Python was only
the external test driver, never an installer dependency.

The script downloaded the release archive and checksum from GitHub, verified the
checksum, and installed CLI, daemon, native library and both global skills. Installed
`--version`, `--agent doctor`, `--agent workflow list` succeeded; both SKILL.md files
matched source. Doctor returned configured=false; no profile, DB or account was created.
The temporary HOME was removed after the check. No user configuration or credentials
were used. Linux was excluded at the user's request.

Local gates before publication: `scripts/check.py verify` (13 checks), release build,
`scripts/test-install.py` (source, bundle, download, reinstall, checksum, unsafe paths,
truncated script, network failure), wiki contract and git diff check. Independent
Fable medium findings and usage are recorded in
[the review report](../../docs/reviews/2026-09-19-agent-onboarding.md).
