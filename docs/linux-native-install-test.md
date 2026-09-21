# Test the native Linux release

The [0.8.0 release](https://github.com/robertelee78/swictation/releases/tag/v0.8.0) is available for this test. No source checkout,
Node runtime, or compiler is required by the new installation. Supported platform:
Linux x86_64 with glibc 2.39 or newer and a user systemd session.

On Ubuntu 24.04, install the [system audio/UI libraries](installation.md#linux-system-libraries)
before setup. Also install your desktop's text-injection tool from the README prerequisites.

## 1. Retire the old npm installation

If Swictation is currently installed through npm, run these while the old command
is still available:

```sh
swictation stop
npm uninstall -g --ignore-scripts swictation
```

Keep the existing Swictation configuration and data directories. Disabling npm's
uninstall scripts prevents its old cleanup hook from deleting service integration.
Fresh installations skip this step.

## 2. Install the published native release

```sh
installer=$(mktemp /tmp/swictation-install.XXXXXX) &&
  curl -fsSL https://github.com/robertelee78/swictation/releases/download/v0.8.0/install.sh -o "$installer" &&
  sh "$installer"
export PATH="$HOME/.local/bin:$PATH"
hash -r
command -v swictation
swictation --version
```

Expected: `~/.local/bin/swictation` and `swictation 0.8.0`. Installation should place
the native release and print the setup command. It should preserve settings and models.

## 3. Set up and test dictation

```sh
swictation setup
swictation doctor --deep
swictation start
swictation status
```

Existing valid model files should be reused; missing or invalid files are downloaded.
Doctor should identify missing host dependencies explicitly. After start, the daemon
should report running and the tray/UI should be available for the desktop in use.

Open a text editor. Use the configured hotkey to start and stop recording, or run
`swictation toggle` in a terminal before and after speaking. Check that the completed
sentence appears in the editor and that your existing preferences are retained.

## 4. Exercise the native update path

```sh
swictation update --check
swictation update --force
swictation status
```

The check should report the published version as current. Force should fetch and
verify the current native archive without changing configuration or model files.
Repeat dictation afterward. Updates must no longer invoke npm.

An actual version upgrade and rollback need two published native versions. This
first release cannot roll back to the retired npm package. Until a previous native
release exists, `swictation update --rollback` should report that none is retained.

## What to report

Report your distribution, desktop/session (X11 or Wayland), GPU, the first failing
command and its complete error, or confirmation that install, setup, dictation and
same-version update all worked. Useful diagnostics are:

```sh
swictation doctor --json
swictation status
swictation logs
```

Uninstall is optional and is not part of the normal dictation test. Running
`swictation uninstall` previews removal without changing anything; `--yes` performs
it. Default uninstall preserves configuration, models and learned data.
