"""Offline installer and release-artifact contracts; no product installation occurs."""
import hashlib
import gzip
import importlib.util
import io
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tarfile
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
VERSION = "0.7.37"
TARGET = "x86_64-unknown-linux-gnu"
ASSET = f"swictation-{VERSION}-{TARGET}.tar.gz"
SOURCE_SHA = "a" * 40
spec = importlib.util.spec_from_file_location("release_package", HERE / "package.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


def executable(path, text):
    path.write_text(text)
    path.chmod(0o755)


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="swictation-bootstrap-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.commands = self.root / "commands"
        self.commands.mkdir()
        (self.root / "downloads").mkdir()
        self.marker = self.root / "installed"
        self.env = dict(os.environ, PATH=f"{self.commands}:{os.environ['PATH']}",
                        FIXTURE=str(self.root), TMPDIR=str(self.root / "downloads"),
                        INSTALL_MARKER=str(self.marker), HOME=str(self.root / "home"))
        executable(self.commands / "uname", '#!/bin/sh\ncase "$1" in -s) echo Linux ;; -m) echo x86_64 ;; esac\n')
        executable(self.commands / "curl", f"""#!{sys.executable}
import os, pathlib, shutil, sys
args = sys.argv[1:]
root = pathlib.Path(os.environ['FIXTURE'])
url = next(arg for arg in args if arg.startswith('https://'))
with (root / 'requests').open('a') as log:
    log.write(url + '\\n')
destination = args[args.index('--output') + 1]
shutil.copyfile(root / url.rsplit('/', 1)[1], destination)
""")
        self.installer = self.root / "install.sh"
        self.candidate = f"""#!/bin/sh
if [ "$1" = --version ]; then printf 'swictation {VERSION}\\n'; exit 0; fi
printf '%s\\n' "$@" > "$INSTALL_MARKER"
cat > "$INSTALL_MARKER.stdin"
exit "${{INSTALL_EXIT:-0}}"
""".encode()
        self.write_archive()

    def write_archive(self, members=None):
        if members is None:
            members = [("bin/swictation", self.candidate, tarfile.REGTYPE)]
        archive = self.root / ASSET
        with tarfile.open(archive, "w:gz") as bundle:
            for name, data, kind in members:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.mode = 0o755
                member.size = len(data) if kind == tarfile.REGTYPE else 0
                if kind in {tarfile.SYMTYPE, tarfile.LNKTYPE}:
                    member.linkname = "../../outside"
                bundle.addfile(member, io.BytesIO(data) if member.size else None)
        checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        Path(str(archive) + ".sha256").write_text(f"{checksum}  {ASSET}\n")
        template = (HERE / "install.sh.in").read_text()
        template = template.replace("@SWICTATION_RELEASE_VERSION@", VERSION)
        template = template.replace("@SWICTATION_APPLE_TEAM_ID@", "ABCDEFGHIJ")
        for prefix in ("LINUX", "MACOS"):
            template = template.replace(f"@SWICTATION_{prefix}_SHA256@", checksum)
            template = template.replace(f"@SWICTATION_{prefix}_SIZE@", str(archive.stat().st_size))
        self.installer.write_text(template)
        return checksum

    def run_installer(self, *arguments, success=False, script=None):
        result = subprocess.run(["/bin/sh", str(script or self.installer), *arguments],
                                env=self.env, input="caller input must not be consumed\n",
                                text=True, capture_output=True)
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(list((self.root / "downloads").iterdir()), [])
        return result

    def test_verified_candidate_receives_exact_archive_and_checksum_with_closed_stdin(self):
        checksum = self.write_archive()
        self.run_installer(success=True)
        arguments = self.marker.read_text().splitlines()
        self.assertEqual(arguments[0:2], ["__install", "--archive"])
        self.assertEqual(Path(arguments[2]).name, ASSET)
        self.assertEqual(arguments[3:5], ["--sha256", checksum])
        self.assertEqual(Path(str(self.marker) + ".stdin").read_text(), "")
        self.assertTrue(all(f"/download/v{VERSION}/" in url for url in (self.root / "requests").read_text().splitlines()))

    def test_matching_explicit_version_keeps_immutable_urls(self):
        self.run_installer("--version", VERSION, success=True)
        urls = (self.root / "requests").read_text().splitlines()
        self.assertEqual(len(urls), 2)
        self.assertTrue(all(f"/download/v{VERSION}/" in url for url in urls))

    def test_version_override_cannot_change_the_rendered_release(self):
        for version in ("latest", "0.7.38", "0.7.37-rc.1", "00.7.37"):
            with self.subTest(version=version):
                self.run_installer("--version", version)
        self.assertFalse((self.root / "requests").exists())
        self.assertFalse(self.marker.exists())

    def test_replacing_archive_and_sidecar_cannot_replace_the_installer_pin(self):
        original_installer = self.installer.read_text()
        self.candidate += b"# replaced payload\n"
        self.write_archive()
        self.installer.write_text(original_installer)
        self.run_installer()
        self.assertFalse(self.marker.exists())

    def test_checksum_mismatch_never_runs_candidate(self):
        (self.root / ASSET).write_bytes((self.root / ASSET).read_bytes() + b"tampered")
        self.run_installer()
        self.assertFalse(self.marker.exists())

    def test_checksum_for_another_filename_is_rejected(self):
        Path(str(self.root / ASSET) + ".sha256").write_text("a" * 64 + "  another.tar.gz\n")
        self.run_installer()
        self.assertFalse(self.marker.exists())

    def test_duplicate_or_linked_candidate_is_rejected(self):
        for members in [
            [("bin/swictation", self.candidate, tarfile.REGTYPE)] * 2,
            [("bin/swictation", b"", tarfile.SYMTYPE)],
            [("bin/swictation", b"", tarfile.LNKTYPE)],
        ]:
            with self.subTest(members=members):
                self.write_archive(members)
                self.run_installer()
                self.assertFalse(self.marker.exists())

    def test_bootstrap_never_extracts_other_members(self):
        outside = self.root / "outside"
        self.write_archive([("bin/swictation", self.candidate, tarfile.REGTYPE),
                            (str(outside), b"must not be written", tarfile.REGTYPE)])
        self.run_installer(success=True)
        self.assertFalse(outside.exists())

    def test_wrong_candidate_version_and_installer_failure_propagate(self):
        self.candidate = self.candidate.replace(VERSION.encode(), b"0.0.0")
        self.write_archive()
        self.run_installer()
        self.assertFalse(self.marker.exists())
        self.candidate = self.candidate.replace(b"0.0.0", VERSION.encode())
        self.write_archive()
        self.env["INSTALL_EXIT"] = "17"
        self.assertEqual(self.run_installer().returncode, 17)

    def test_unsupported_platform_and_unsafe_version_fail_before_network(self):
        executable(self.commands / "uname", '#!/bin/sh\necho unsupported\n')
        self.run_installer()
        self.assertFalse((self.root / "requests").exists())
        self.run_installer("--version", "../../wrong")
        self.assertFalse(self.marker.exists())

    def test_truncated_stream_does_not_execute(self):
        truncated = self.root / "truncated.sh"
        truncated.write_text(self.installer.read_text().rsplit("fi", 1)[0])
        self.run_installer(script=truncated)
        self.assertFalse((self.root / "requests").exists())


class PackagingTests(unittest.TestCase):
    def test_complete_archive_dereferences_libraries_and_rejects_identity_drift(self):
        with tempfile.TemporaryDirectory(prefix="swictation-package-test-") as temporary:
            root = Path(temporary)
            libraries = root / "lib"
            libraries.mkdir()
            (libraries / "libonnxruntime.so.1").write_bytes(b"test runtime")
            (libraries / "libonnxruntime.so").symlink_to("libonnxruntime.so.1")
            for name in ("libonnxruntime_providers_cuda.so", "libonnxruntime_providers_shared.so"):
                (libraries / name).write_bytes(b"test provider")
            cli = root / "swictation"
            executable(cli, f"#!/bin/sh\necho 'swictation {VERSION}'\n")
            (root / "config.toml").write_text("# config fixture\n")
            (root / "models.json").write_text('{"models": []}\n')
            (root / "tray.py").write_text("# tray fixture\n")
            (root / "logo.png").write_bytes(b"logo fixture")
            subprocess.run([sys.executable, str(HERE / "package.py"), "stage", "--target", TARGET,
                            "--source-sha", SOURCE_SHA, "--cli", str(cli), "--daemon", str(cli),
                            "--ui", str(cli), "--lib-dir", str(libraries), "--stage-dir", str(root / "stage"),
                            "--config", str(root / "config.toml"), "--models-manifest", str(root / "models.json"),
                            "--tray", str(root / "tray.py"), "--logo", str(root / "logo.png")],
                           check=True, capture_output=True)
            subprocess.run([sys.executable, str(HERE / "package.py"), "archive", "--stage-dir", str(root / "stage"),
                            "--output-dir", str(root / "output")], check=True, capture_output=True)
            archive = root / "output" / ASSET
            metadata = {"schema_version": 1, "version": VERSION, "target": TARGET, "source_sha": SOURCE_SHA}
            package.verify_payload(archive, metadata)
            with tarfile.open(archive) as bundle:
                self.assertTrue(all(member.isfile() or member.isdir() for member in bundle))
                self.assertEqual(bundle.extractfile("lib/libonnxruntime.so").read(), b"test runtime")
                self.assertEqual(json.load(bundle.extractfile("release.json")), metadata)
            with self.assertRaisesRegex(ValueError, "metadata differs"):
                package.verify_payload(archive, dict(metadata, source_sha="b" * 40))

    def test_renderer_requires_exact_team_and_version(self):
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "install.sh"
            command = [sys.executable, str(HERE / "render-installer.py"), "--version", VERSION,
                       "--output", str(destination), "--distribution-dir", temporary]
            result = subprocess.run(command, env=dict(os.environ, APPLE_TEAM_ID=""), capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(destination.exists())
            for target in package.TARGETS:
                asset = Path(temporary) / f"swictation-{VERSION}-{target}.tar.gz"
                asset.write_bytes(b"renderer fixture " + target.encode())
                digest = hashlib.sha256(asset.read_bytes()).hexdigest()
                Path(str(asset) + ".sha256").write_text(f"{digest}  {asset.name}\n")
            subprocess.run(command, env=dict(os.environ, APPLE_TEAM_ID="ABCDEFGHIJ"), check=True)
            self.assertNotIn("@SWICTATION_", destination.read_text())
            subprocess.run(["/bin/sh", "-n", str(destination)], check=True)

    def test_stable_release_identity_rejects_prerelease_and_leading_zero(self):
        metadata = {"schema_version": 1, "version": VERSION, "target": TARGET, "source_sha": SOURCE_SHA}
        for version in ("0.7.37-rc.1", "01.2.3", "1.02.3", "1.2.03", "1.2.3+build"):
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, "invalid release version"):
                package.checked_metadata(dict(metadata, version=version))

    def test_expanded_native_limit_is_enforced_before_reading_large_member(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / ASSET
            member = tarfile.TarInfo("lib/oversized.so")
            member.size = 4 * 1024**3 + 1
            archive.write_bytes(gzip.compress(member.tobuf() + b"\0" * 1024))
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            Path(str(archive) + ".sha256").write_text(f"{digest}  {archive.name}\n")
            metadata = {"schema_version": 1, "version": VERSION, "target": TARGET, "source_sha": SOURCE_SHA}
            with self.assertRaisesRegex(ValueError, "archive exceeds release limits"):
                package.verify_payload(archive, metadata)

    def test_macos_stage_contains_complete_daemon_and_ui_app_bundles(self):
        with tempfile.TemporaryDirectory(prefix="swictation-macos-package-test-") as temporary:
            root = Path(temporary)
            (root / "lib").mkdir()
            (root / "lib/libonnxruntime.dylib").write_bytes(b"runtime fixture")
            cli = root / "swictation"
            executable(cli, f"#!/bin/sh\necho 'swictation {VERSION}'\n")
            (root / "config.toml").write_text("# config fixture\n")
            (root / "models.json").write_text('{}\n')
            (root / "tray.py").write_text("# tray fixture\n")
            (root / "logo.png").write_bytes(b"logo fixture")
            app = root / "Swictation.app"
            (app / "Contents/MacOS").mkdir(parents=True)
            executable(app / "Contents/MacOS/swictation-ui", "#!/bin/sh\nexit 0\n")
            (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
                "CFBundleIdentifier": "com.swictation.ui", "CFBundleExecutable": "swictation-ui"}))
            subprocess.run([sys.executable, str(HERE / "package.py"), "stage", "--target", package.TARGETS[1],
                            "--source-sha", SOURCE_SHA, "--cli", str(cli), "--daemon", str(cli),
                            "--ui", str(cli), "--lib-dir", str(root / "lib"), "--stage-dir", str(root / "stage"),
                            "--config", str(root / "config.toml"), "--models-manifest", str(root / "models.json"),
                            "--tray", str(root / "tray.py"), "--logo", str(root / "logo.png"), "--app", str(app)],
                           check=True, capture_output=True)
            daemon_info = plistlib.loads((root / "stage/share/SwictationDaemon.app/Contents/Info.plist").read_bytes())
            self.assertEqual(daemon_info["CFBundleIdentifier"], "com.swictation.daemon")
            self.assertTrue(daemon_info["NSMicrophoneUsageDescription"])
            self.assertEqual((root / "stage/share/SwictationDaemon.app/Contents/MacOS/swictation-daemon").read_bytes(), cli.read_bytes())
            subprocess.run([sys.executable, str(HERE / "package.py"), "archive", "--stage-dir", str(root / "stage"),
                            "--output-dir", str(root / "output")], check=True, capture_output=True)


if __name__ == "__main__":
    unittest.main()
