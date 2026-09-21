# Standalone release tooling

Swictation publishes native archives and an immutable shell installer through GitHub Releases.
Python and Node are build tools; Node is used only to build the Tauri frontend.
The installed command and its installation/update lifecycle are native Rust.

## macOS release credentials

The macOS workflow builds and tests without Apple credentials. Official builds embed the
public signing team `3T2D2YNTVW` in the native CLI. Their unsigned payload is transferred to
a separate signing job using GitHub's protected `apple-release` environment. That job
checks the source commit, archive digest, and payload metadata before signing the same
bytes. Ordinary CI packages stay unsigned and do not enter that environment.

Configure these **environment variables** in Swictation's own `apple-release`
environment. The variable names follow hf2q's workflow; their values are configured
independently for this repository:

| Variable | Value |
| --- | --- |
| `APPLE_DEVELOPER_ID_APPLICATION` | Full Developer ID Application identity, including `(3T2D2YNTVW)` |
| `APPLE_NOTARY_KEY_ID` | App Store Connect API key ID |
| `APPLE_NOTARY_ISSUER_ID` | App Store Connect API issuer UUID |

Configure these **environment secrets** in the same environment:

| Secret | Value |
| --- | --- |
| `APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64` | Base64-encoded Developer ID certificate/private-key archive |
| `APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD` | Password for that P12 archive |
| `APPLE_NOTARY_KEY_P8_BASE64` | Base64-encoded App Store Connect private API key |

Keep private material outside the repository. The signing script does not print it and
removes temporary credential files when it exits. Apple ID and app-specific passwords
are not required. The repository's environment protections control access to signing;
these scripts do not create or modify those protections.

The Developer ID certificate identifies the publisher and can sign multiple apps.
Swictation has its own `com.swictation.cli`, `com.swictation.daemon`, and
`com.swictation.ui` identifiers. A separate notarization API key is recommended for
Swictation CI so it can be rotated or revoked without disrupting another project.
Apple team API keys are not restricted to a single app; independent keys provide
independent revocation, not per-app access isolation. See [Apple's API key guide](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api).

The signing job signs libraries, native executables, and both complete app bundles;
notarizes the payload; staples both apps; and checks the resulting signatures and tickets.
It then archives and smoke-tests the exact signed package before uploading the final
`swictation-aarch64-apple-darwin` artifact. Publishing depends on this job succeeding.

Tauri uses `$RUNNER_TEMP/ui-target` to avoid the repository's tracked default target tree.

## Local signing with an existing keychain identity

Use an already installed Developer ID identity and an existing notary keychain profile:

```sh
export APPLE_TEAM_ID=3T2D2YNTVW
export APPLE_DEVELOPER_ID_APPLICATION='Developer ID Application: ROBERT E LEE (3T2D2YNTVW)'
bash scripts/distribution/sign-macos.sh /absolute/path/to/staged-payload \
  --local-keychain --keychain-profile your-existing-notary-profile
```

`APPLE_NOTARY_KEYCHAIN_PROFILE` can supply the profile instead of the argument. An optional
`--keychain /absolute/path/to/existing.keychain-db` scopes code signing to a particular
keychain. Local mode imports no certificate and leaves existing keychains and their
search lists unchanged. It can also use the three notary API-key environment values
above instead of a profile. Build the official native CLI with
`SWICTATION_APPLE_TEAM_ID=3T2D2YNTVW` before staging a locally signed release.

## Archive and installer contracts

`package.py stage` assembles binaries, dereferenced libraries, complete macOS bundles,
configuration/model metadata, and the wlroots tray assets. `package.py archive` creates
`swictation-VERSION-TARGET.tar.gz` plus its SHA-256 sidecar. `package.py verify` checks
the exact source/version/target, safe member paths, required assets, and native size limits.

After both target archives have been verified, render the release installer:

```sh
APPLE_TEAM_ID=3T2D2YNTVW python3 scripts/distribution/render-installer.py \
  --version VERSION --distribution-dir /absolute/path/to/archives \
  --output /absolute/path/to/archives/install.sh
```

The rendered installer pins both archives' SHA-256 values and byte lengths. Its optional
`--version` argument may only repeat the pinned version. To select another version, use
that release's installer; `releases/latest/download/install.sh` selects the latest one.

`smoke-install.py` exercises the packaged native command in an isolated home without
configuring services. Unsigned local CI fixtures require `--allow-unsigned-local`;
official signed packages run without that bypass.
