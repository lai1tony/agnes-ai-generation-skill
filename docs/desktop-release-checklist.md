# Agnes AI Studio Release Checklist

## Verified on Windows

- `npm run build`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run test:ui`
- `npm run verify`
- `npm run tauri -- build --bundles nsis`

Windows installer output:

- `src-tauri/target/release/bundle/nsis/Agnes AI Studio_0.1.0_x64-setup.exe`

## macOS Readiness

- Tauri bundle target includes `dmg`.
- CI matrix includes `macos-latest` with `--bundles dmg --no-sign`.
- CI runs `npm run verify` before packaging, covering frontend build, Rust backend tests, and Playwright UI smoke.
- CI verifies Windows NSIS installer presence and verifies macOS DMG images with `hdiutil verify` before uploading artifacts.
- Rust backend tests cover Agnes text/image/video response handling plus image edit, text-to-video, image-to-video, multi-image video, and keyframe video payload shapes.
- macOS icon resource exists at `src-tauri/icons/icon.icns` and contains standard PNG-backed entries from 16x16 through 1024x1024.
- Windows icon resource exists at `src-tauri/icons/icon.ico` and contains 16, 32, 64, 128, and 256 px entries.
- Cargo target metadata resolves the macOS secure storage backend:
  - `x86_64-apple-darwin`: `keyring` + `security-framework`, no `windows-sys`.
  - `aarch64-apple-darwin`: `keyring` + `security-framework`, no `windows-sys`.
- Windows-host `cargo check --target *-apple-darwin` reaches Apple Objective-C crate compilation and then fails because this host has no Apple `cc`; run the final DMG build on macOS.

## Remaining Proof Needed

Run the `Desktop build` GitHub Actions workflow, or run this on a macOS machine:

```bash
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build:mac
```

The goal is fully verified when the CI macOS job passes `hdiutil verify` and a DMG artifact exists under:

```text
src-tauri/target/release/bundle/dmg/*.dmg
```
