# Tauri UI development and release builds

Updated: 2026-09-21. Node/npm are development dependencies of this frontend. Installed
Swictation uses the native lifecycle in [ADR-038](../../docs/adr/ADR-038-native-distribution-lifecycle.md)
and requires no JavaScript interpreter or npm product package.

## Local development

From `tauri-ui/`:

```sh
npm ci
npm run tauri dev
```

Tauri embeds the built frontend in the application. A stale `dist/` or bundle directory
can make a newly built executable contain old frontend assets; inspect the actual
bundled output when investigating mismatched UI events.

## Release UI build

From `tauri-ui/`:

```sh
./scripts/build-ui-release.sh
```

The script builds and validates the UI. It does not copy a binary into an npm package
or publish a product. Follow the current script's output for artifact paths and failures.
Frontend compilation and static asset checks do not substitute for installed UI testing.

For a manual development check:

```sh
npx tsc --noEmit
npm run build
npm run tauri build
```

On macOS, preserve the `.app` bundle and nested resources. The native release workflow
must sign and notarize the final components and app; merely building a local app/DMG
is not evidence of release trust or a complete Swictation installation.

## Product assembly and validation

`scripts/distribution/package.py` at repository root assembles the native release from
exact CLI, daemon, UI, libraries and setup inputs. The release bundle records its
source commit, version and target. The generated installer pins the resulting archive;
there is no npm `prepublishOnly` hook or registry publication step.

Before publication, prove the UI from the packaged installed release on each supported
host: tray activation, status/toggle IPC, live metrics, settings and learned corrections.
Check that service paths follow the current release, and repeat after update/rollback.
Do not infer user-visible functionality from event strings embedded in a binary.

Use the [release checklist](../../docs/RELEASE_CHECKLIST.md) for exact-artifact and
host proof. The first native release remains pending until those publication and
clean-host requirements are met.
