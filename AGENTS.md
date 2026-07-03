# AGENTS.md — Datlino

The build/test/release contract for this repo. An agent (or the overnight ralph loop) should be
able to read only this file and ship correctly. Keep every command copy-pasteable and current.
Taste rules that apply to every flywheel product live in the hub: `flywheel/docs/standards/taste.md`.

> One-liner: macOS touch-typing trainer that drills students on their own study materials.
> Stack/template: Tauri 2 (Rust) + SvelteKit (static adapter)  ·  Track: desktop  ·  Portfolio record: `flywheel/data/products/datlino.json`

## Build
```bash
npm ci                       # frontend deps (Rust deps fetch on first cargo/tauri run)
npm run build                # SvelteKit static build → build/
npx tauri build              # native app bundle (.dmg/.app) into src-tauri/target/*/release/bundle
```

## Test (TDD required; persona-journey test per primary journey)
```bash
npm run check                # svelte-kit sync + svelte-check (typecheck)
cargo test --workspace --manifest-path src-tauri/Cargo.toml   # Rust unit + persona tests
npm run research:pw          # Playwright persona-visual e2e; writes committed screenshots
```
Gate: typecheck · test · build must pass (CI is `docs/standards/ci.template.yml`). Block only on these.
There is no `npm test` script — `npm run check` is the typecheck gate and `cargo test` is the test gate.

## Run / verify a change in the real app
```bash
npm run tauri dev            # launches the native window with hot-reload (Vite on :1420)
```
Look at the two doors on `/` — **Učím se psát** (typing course) and **Učím se obsah** (drill your
own notes). The `/run` skill covers this stack.

## Release (the finish line — produces a storefront link)
- **Desktop** → signed + notarized DMG on a GitHub Release. Push a tag `vX.Y.Z`; `.github/workflows/release.yml`
  builds the macOS/Linux/Windows matrix and uploads artefacts. Signing + notarization are guarded on
  the Apple secrets being present (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`); without them the pipeline still produces unsigned artefacts.
  Latest: [v0.1.0](https://github.com/Chartres/datlino/releases/tag/v0.1.0) (`Datlino_0.1.0_aarch64.dmg`, Apple Silicon).
- Adoption is read from the GitHub release **download counts** — no extra telemetry needed.

## Analytics (Common Platform)
Vendored client: `src/lib/flywheel.ts` (dependency-free fetch into the shared flywheel-core Supabase project).
Consent-gated — off until the user opts in (About page promises no telemetry without consent); ships dark
when `VITE_FLYWHEEL_KEY` is unset. Fires the shared taxonomy (`page_view`, `key_action`, `conversion`,
`feedback_given`, `error`); no note content ever leaves the device. The aha moment is a finished typing
session (`sessionCompleted` → `conversion`), matching `activation_event: "conversion"` in the portfolio record.
Platform env: `VITE_FLYWHEEL_URL`, `VITE_FLYWHEEL_KEY`.

## Done means
Green CI · released (notarized GitHub release) · portfolio record updated (stage/gate/links) ·
storefront link live · (outward promotion only after Pavol's sign-off).
