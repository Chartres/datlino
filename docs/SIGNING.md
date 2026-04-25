# Signing & notarization (DIS-001)

Datlino ships as native installers on macOS, Windows, and Linux. The
GitHub Actions `release.yml` workflow builds unsigned artefacts out of
the box and becomes a signing + notarization pipeline once the secrets
below are populated. Each step is opt-in — the build still runs on
forks and PRs without certificates.

## macOS

1. **Apple Developer account.** Paja Dravec's personal-developer or a
   Datlino s.r.o. team membership. ~$99/yr.
2. **Developer-ID Application certificate.** Generate via
   Xcode → Settings → Accounts → Manage Certificates, export as a
   `.p12` with a password. Base64 it:
   ```
   base64 -i DevIDApp.p12 | pbcopy
   ```
3. **App-specific password** for notarization. Generate at
   [appleid.apple.com](https://appleid.apple.com) → Sign-In and
   Security → App-Specific Passwords.
4. **GitHub secrets** to set:
   - `APPLE_CERTIFICATE` — base64 of the .p12
   - `APPLE_CERTIFICATE_PASSWORD` — the .p12 password
   - `APPLE_SIGNING_IDENTITY` — full identity string, e.g.
     `Developer ID Application: Paja Dravec (AB12CD34EF)`
   - `APPLE_ID` — your Apple ID email
   - `APPLE_PASSWORD` — the app-specific password from step 3
   - `APPLE_TEAM_ID` — the 10-char team ID (Xcode → Accounts)

Once present the workflow signs the `.app`, packages a notarized
`.dmg`, and staples the notarization ticket. Verify after download
with `spctl --assess --type open --context context:primary-signature Datlino.dmg`.

## Windows

1. **Code-signing certificate.** Easiest route is an **EV** cert from
   DigiCert / Sectigo (~$300–600/yr) — an EV cert bypasses SmartScreen
   warnings on day one. A standard OV cert (~$100–300/yr) works but
   builds SmartScreen reputation over weeks.
2. Export the certificate as `.pfx` with a password, base64 it:
   ```
   certutil -encode cert.pfx cert.pfx.txt
   ```
3. **GitHub secrets**:
   - `WINDOWS_CERTIFICATE` — base64 of the .pfx
   - `WINDOWS_CERTIFICATE_PASSWORD` — the .pfx password

Tauri's bundler picks up the cert from the keystore and calls
`signtool` automatically.

## Linux

`.AppImage` and `.deb` don't require signing for Datlino's use case —
students download from the GitHub Release and run. If we eventually
ship to Flathub or Snapcraft, that's a separate signing story each.

## Tauri auto-updater (DIS-002)

The workflow also builds the `.sig` files Tauri's updater needs. Two
more secrets when you're ready:

- `TAURI_UPDATER_SIGNING_KEY` — private key from
  `npx tauri signer generate`
- `TAURI_UPDATER_SIGNING_KEY_PASSWORD` — the passphrase

The workflow's `release` job publishes `latest.json` to the
`updater-endpoint` branch; point `tauri.conf.json > plugins > updater
> endpoints` at the `raw.githubusercontent.com` URL for it. Students
on older versions then see an auto-update prompt on next launch.

## What you actually need to do

- [ ] Enrol Apple Developer, generate + export the Developer-ID cert
- [ ] Buy a Windows code-signing cert (EV recommended)
- [ ] Run `npx tauri signer generate` locally once, copy the public
      key into `tauri.conf.json` and the private key into the GitHub
      secret
- [ ] Fill in the seven GitHub secrets above
- [ ] Tag `v0.1.0` → the workflow picks it up and cuts your first
      signed+notarized Release

Until then, every push to `release/**` produces unsigned artefacts
under the workflow run's Artifacts tab — good for internal dogfooding.
