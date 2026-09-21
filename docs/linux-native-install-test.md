# Test the native Linux release

Test the latest native release on Linux x86_64 with glibc 2.39 or newer and a
user systemd session. No source checkout, Node runtime, or compiler is required.

On Ubuntu 24.04, install the [system audio/UI libraries](installation.md#linux-system-libraries)
before setup. Also install your desktop's text-injection tool from the README prerequisites.

## 1. Install the published native release

```sh
curl -fsSL https://github.com/robertelee78/swictation/releases/latest/download/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
hash -r
command -v swictation
swictation --version
```

Expected: `~/.local/bin/swictation` and the latest published version. Installation should place
the native release and print the setup command. It should preserve settings and models.

## 2. Set up and test dictation

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

## 3. Exercise the native update path

```sh
swictation update --check
swictation update --force
swictation status
```

The check should report the published version as current. Force should fetch and
verify the current native archive without changing configuration or model files.
Repeat dictation afterward. Updates must no longer invoke npm.

If you upgraded from an earlier native release, `swictation update --rollback`
should restore that retained release. On a fresh installation it should report
that no previous release is retained.

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
