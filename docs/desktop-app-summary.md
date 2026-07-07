# Agnes AI Studio Desktop App Summary

## What Changed

- Added a Tauri v2 + React + TypeScript desktop app while keeping the original Agent Skill files and Python CLI.
- Moved Agnes API calls into the Rust backend so the frontend never reads the API key.
- Added Chinese-first desktop workspaces for text, image, video, history, and settings.
- Added system secure storage for the Agnes API key through platform keyring backends.
- Added manual media saving for generated image/video URLs.
- Added Windows NSIS and macOS DMG bundle configuration.
- Added GitHub Actions CI for Windows and macOS packaging.

## Agnes Capabilities Covered

- Text generation.
- Text-to-image.
- Image-to-image / image editing.
- Text-to-video.
- Image-to-video.
- Multi-image video.
- Keyframe video.
- Video task lookup and polling.
- Non-English image/video prompt translation before generation.

## Verification Commands

```bash
npm run verify
```

Runs:

- TypeScript + Vite frontend build.
- Rust backend tests.
- Playwright UI smoke test.

Platform packaging:

```bash
npm run tauri:build:windows
npm run tauri:build:mac
```

The macOS command creates an unsigned DMG for local validation. Use `npm run tauri:build:mac:signed` only after configuring Developer ID signing.

## Current Local Evidence

- Windows verification passed with `npm run verify`.
- Windows NSIS package build passed with `npm run tauri -- build --bundles nsis`.
- Windows installer exists at:

```text
src-tauri/target/release/bundle/nsis/Agnes AI Studio_0.1.0_x64-setup.exe
```

## Remaining macOS Proof

This Windows machine cannot produce or verify a DMG. To finish the cross-platform proof, run the `Desktop build` GitHub Actions workflow after pushing these changes, or run this on macOS:

```bash
npm ci
npm run verify
npm run tauri:build:mac
hdiutil verify src-tauri/target/release/bundle/dmg/*.dmg
```
