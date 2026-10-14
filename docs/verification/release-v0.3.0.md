# Windows release verification — v0.3.0

The supplied `assets/logos/rc-logo-transparant-bg.png` is the icon source.
The conversion preserves its artwork, colours and transparent background.
The originals remain in `assets/logos/`; `examples/make_icons.rs` reproduces
the 256px runtime PNG and the nine-frame Windows ICO.

## Verified before packaging

| Check | Result |
| --- | --- |
| `cargo test --release --locked` | 46 passed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed |
| `cargo build --release --locked --target x86_64-pc-windows-msvc --bin rusted-calamari` | Passed |
| Native workspace smoke with the new runtime icon | Passed all phases, including detached trees and Anchor |
| Executable architecture and subsystem | Windows x64, GUI subsystem |
| Windows version resource | Product name Rusted Calamari; product version 0.3.0 |
| Embedded icon payloads | All nine match the generated ICO: 16, 20, 24, 32, 40, 48, 64, 128 and 256px |
| Actual release executable launch | Native window and non-null runtime icon verified using an isolated profile and fixture library; normal exit code 0 |
| Portable ZIP preflight | Archive and extracted executable checksums verified; extracted executable passed the same resource and native launch checks |

Local evidence is kept under ignored `target/`:

- `icon-workspace-smoke-20261013/`: native smoke screenshots and `RESULT.txt`.
- `icon-executable-verified-20261013/RESULT.json`: executable hash, resource
  comparison, architecture, version and native launch result.
- `package-preflight-launch-20261013/RESULT.json`: the same checks against
  the executable extracted from the packaging script's ZIP.

The runtime PNG is shared by the main window and both floating trees. The
Windows build embeds the ICO and version resources using
[winresource](https://docs.rs/winresource/0.1.34/winresource/).

## Packaging and publication gate

Run `scripts/package-release.ps1` from Windows PowerShell 7. It builds the
explicit Windows MSVC host target and records version, source revision,
working-tree state, compiler and executable checksum in `BUILD-INFO.json`.
The ZIP includes the executable, user documentation, icon/source artwork,
font licence and dependency notices. A separate SHA-256 file covers the ZIP.
Outputs are never overwritten and stay under ignored `target/packages/`.

Before publication, extract the ZIP into a fresh directory, compare its
executable hash with `BUILD-INFO.json`, and rerun:

```powershell
pwsh -File scripts/verify-release.ps1 -Executable <extracted-exe> -EvidenceDirectory <fresh-directory>
```

Publish the ZIP and checksum as GitHub release assets for the versioned tag;
do not commit generated executables or ZIPs into Git.

## Evidence boundary

Resource checks inspect the actual PE executable; launch checks use the
actual distributable with an isolated Windows profile. The workspace smoke
drives application actions and records native geometry. It does not provide
physical mouse, mixed-DPI or multi-monitor acceptance, nor visual acceptance
of Explorer/taskbar icon caching on another machine. Earlier workspace
limitations remain documented in [the workspace report](workspace-2026-10-13.md).
