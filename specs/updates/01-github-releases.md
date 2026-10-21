# Desktop app updates from GitHub Releases

## Runtime behavior

- Tauri's updater checks the public static manifest at `https://github.com/duongonix/recodex/releases/latest/download/latest.json`.
- The updater compares the signed manifest's SemVer version with the version in `src-tauri/tauri.conf.json`; it returns an update only when the release is newer.
- Release builds check on startup, when the app regains focus (throttled to one check per five minutes), and every six hours. Development builds do not query releases or offer an update.
- The titlebar shows the download button immediately to the left of Info only while a newer update is available. Clicking downloads, verifies the signature, installs, and relaunches where the platform requires it.
- GitHub Releases must be public and publish a valid `latest.json` with signed updater artifacts for each supported target. A release containing only normal installer assets is not sufficient.

## Signing key

Tauri verifies update signatures before installation. The public key is embedded in `src-tauri/tauri.conf.json`. The generated private key is kept locally at `src-tauri/recodex-updater.key`, which is ignored by Git.

The release workflow is `.github/workflows/release.yml`. Before the first tagged release:

1. Add a repository Actions secret named `TAURI_SIGNING_PRIVATE_KEY` containing the private key file's contents.
2. Keep the private key out of commits, artifacts, logs, and release assets. Do not replace it after users install a signed release; replacing it requires a new app installation for users to trust a different key.
3. The workflow uses `GITHUB_TOKEN` with `contents: write` to upload bundles and update the release manifest; no separately created GitHub token is needed.
4. Keep the generated public key in `src-tauri/tauri.conf.json` paired with the private key secret. The workflow publishes the release only after all six matrix targets succeed.

The current generated key has no password. If it is later replaced with a password-protected key, configure `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as a separate Actions secret.

## Release workflow and target matrix

Push a version tag such as `v0.1.0` to `duongonix/recodex` after setting the same version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`. The workflow rejects runs from a different repository, checks the version values, runs frontend checks/lint/tests/build, and then publishes six architecture targets:

| OS | Target | Release bundle | In-app updater |
|---|---|---|---|
| Windows | x64 | NSIS installer | Yes |
| Windows | ARM64 | NSIS installer | Yes |
| macOS | Intel x64 | DMG + signed updater archive | Yes |
| macOS | Apple Silicon ARM64 | DMG + signed updater archive | Yes |
| Linux | x64 | AppImage, DEB, RPM | AppImage |
| Linux | ARM64 | AppImage, DEB, RPM | AppImage |

The builds use native GitHub-hosted runners for each target. Linux ARM AppImage packaging is the least-tested matrix entry; Tauri documents a slower QEMU-based build path for Linux ARM AppImages if the native runner's bundler tooling cannot build it. Release automation has not yet run against GitHub, so the six artifacts, updater manifest, and update installation remain unverified until a tagged release succeeds.

Windows releases use NSIS as the updater installer format. macOS builds use ad-hoc signing when no Apple Developer certificate is configured; this is not notarization. Windows binaries are not Authenticode-signed unless a code-signing setup is added. macOS/Linux capture adapters are still not implemented in the app, so producing installers does not mean those capture flows are supported.

For public downloads and the configured `latest.json` endpoint, the GitHub repository/release must be public. Users install Linux DEB/RPM packages through their package manager; Tauri's updater applies to Linux AppImage installations.
