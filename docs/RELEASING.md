# Releasing Glean

This document describes how to cut a new release of Glean for macOS.

## Prerequisites

### 1. Apple Developer Certificate (optional, for signed builds)

Without signing, users need to run `xattr -dr com.apple.quarantine /Applications/Glean.app` on first launch.

To sign:
1. Enroll in [Apple Developer Program](https://developer.apple.com/programs/) ($99/year)
2. Obtain a "Developer ID Application" certificate
3. Set environment variables before build:
   ```bash
   export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (XXXXXXXXXX)"
   export APPLE_ID="you@example.com"
   export APPLE_PASSWORD="app-specific-password"  # see appleid.apple.com
   export APPLE_TEAM_ID="XXXXXXXXXX"
   ```

### 2. Tauri Updater Keypair (for auto-update)

Generate a keypair once and store the private key as a GitHub Repository Secret:

```bash
pnpm tauri signer generate -w ~/.tauri/glean.key
```

Output:
```
Public Key: <PUBKEY>      # paste into tauri.conf.json plugins.updater.pubkey
Private Key: <PRIVATE>    # add as GitHub Secret TAURI_SIGNING_PRIVATE_KEY
Password:   <PASSWORD>    # add as GitHub Secret TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

## Cutting a Release

### Option A: Tag-triggered (recommended)

```bash
# 1. Update version in src-tauri/tauri.conf.json and package.json
# 2. Commit and push
git commit -am "chore: bump version to v0.1.0"
git push

# 3. Create tag and push
git tag v0.1.0
git push origin v0.1.0
```

The `release.yml` workflow will:
1. Build for both `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. Sign the bundles with `TAURI_SIGNING_PRIVATE_KEY`
3. Create a draft GitHub Release with `.dmg` / `.app.tar.gz` / `latest.json` assets
4. Publish the Release manually after review

### Option B: Manual dispatch

Go to Actions → Release → Run workflow → input tag name.

## After Release

1. Review the draft Release on GitHub
2. Edit release notes (changelog, screenshots, known issues)
3. Click "Publish release"
4. Verify `latest.json` is uploaded — this is what the auto-updater polls
5. Announce on:
   - V2EX / 即刻 / 小红书
   - Product Hunt (English market)
   - Hacker News (Show HN)

## Verifying Auto-Update

After publishing, users on the previous version should see an update dialog within an hour (the app polls `latest.json` on launch and periodically).

To force-check, run in terminal:
```bash
curl -s https://github.com/WalkAlone0325/glean/releases/latest/download/latest.json | jq .
```

## Troubleshooting

### Build fails: "no signing identity"
Either provide signing env vars, or set `signingIdentity: null` in tauri.conf.json (unsigned build).

### Updater: "signature verification failed"
The `TAURI_SIGNING_PRIVATE_KEY` GitHub Secret doesn't match the `pubkey` in tauri.conf.json. Regenerate and re-upload both.

### macOS: "App is damaged"
This happens when Gatekeeper quarantines an unsigned app. Instruct users to run `xattr -dr`.
