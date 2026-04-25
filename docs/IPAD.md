# Datlino on iPad (PLT-001)

Tauri 2 has first-class iOS support via `@tauri-apps/cli`. Datlino's
codebase already compiles for `aarch64-apple-ios`; this doc is the
bridge from "it compiles" to "it's on Tereza's iPad".

## Status

* ✅ Rust library compiles for `aarch64-apple-ios-sim`
* ✅ SvelteKit adapter-static produces the SPA bundle iOS consumes
* ✅ Shared state + schema migrations work on iOS SQLite
* 🟡 Keyboard + typing surface work but need touch-friendly spacing
* 🟡 Folder-picker via `plugin-dialog` — iOS uses `UIDocumentPickerViewController`,
      needs a capability stanza
* ⛔️ Candle local embeddings — the `candle` feature pulls in
      `ort`-style native deps we haven't verified on iOS yet; ship
      with Fake or Cohere on iPad until Candle iOS path is validated
* ⛔️ OCR — depends on `tesseract` + `pdftoppm` system binaries that
      don't exist on iOS. On iPad, route to Apple's `VNRecognizeText`
      (Vision framework) via a tiny Swift bridge

## One-time setup on your Mac

```sh
# Install Xcode 16+ from the App Store, then:
xcode-select --install
sudo xcodebuild -license
brew install cocoapods
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

## Initialize the iOS target

From the repo root:

```sh
npx tauri ios init
```

This generates `src-tauri/gen/apple/` with the Xcode project, Info.plist,
entitlements, and signing config. Datlino's `tauri.conf.json` already
carries the bundle identifier `org.datlino.app`, so `tauri ios init`
picks that up as the iOS bundle ID.

## Dev loop

```sh
# Simulator (no provisioning needed)
npx tauri ios dev

# Real iPad — requires an Apple Developer team signed into Xcode
npx tauri ios dev --host
# then pick your iPad from the device list
```

First build will take 15–20 min (Candle + tokenizers compile for arm64
once), subsequent builds are incremental.

## iPad-specific adaptations

The SvelteKit SPA already renders on iPad Safari, but a few things
deserve iPad treatment before shipping:

1. **Soft-keyboard handling** — when the system keyboard appears the
   typing surface gets pushed. Use `visual-viewport` API to reflow
   the on-screen finger keyboard off-screen automatically (it's
   redundant when the iOS keyboard is up).
2. **Touch gestures** — the document picker row already works; the
   α-slider on `/study` needs a wider thumb for finger precision.
3. **Landscape vs portrait** — the two-doors home page collapses
   nicely in portrait. The typing surface reads better in landscape.
4. **GoodNotes import** — since Tereza lives in GoodNotes, we
   register Datlino as a "Share Sheet" target for PDF so she can
   export→Datlino in one tap. Add `LSSupportsOpeningDocumentsInPlace`
   and `CFBundleDocumentTypes` to the generated Info.plist after
   `tauri ios init`.

## What's gated on this

* **AI-004** — Candle download progress UI needs a resumable-download
  story on iOS (iOS kills backgrounded network tasks aggressively)
* **ING-007** — Apple Vision OCR becomes the *default* iPad OCR path
  (free, on-device, first-party CZ/SK models)
* **DIS-001** — for TestFlight distribution add a separate workflow
  matrix row with `xcrun altool --upload-app`; App Store submission
  needs privacy-manifest + App Review

## Why iPad matters

Tereza's GoodNotes flow lives there (+3 persona vote). Pája writes
slohovky on her iPad in Split View (+2). The whole PLT-001 /
PLT-002 trio ties the CZ/SK school-device reality together (school-
provided iPads are common in the gymnázium + maturita cohort).
