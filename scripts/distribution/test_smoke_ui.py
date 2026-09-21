"""UI smoke cleanup contracts; never start a UI or touch real mounts/services."""
import io
from contextlib import redirect_stdout
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

import smoke_ui


class PortalCleanupTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="swictation-ui-cleanup-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.runtime = self.root / "runtime with space"
        self.runtime.mkdir(mode=0o700)

    def mountinfo(self, path=None, filesystem="fuse.portal"):
        path = str(path or self.runtime / "doc")
        escaped = path.replace("\\", r"\134").replace(" ", r"\040")
        escaped = escaped.replace("\t", r"\011").replace("\n", r"\012")
        return f"42 21 0:55 / {escaped} rw,nosuid,nodev shared:3 - {filesystem} portal rw\n"

    def test_other_portals_and_unmounted_doc_are_untouched(self):
        mounts = self.mountinfo(Path("/run/user/1000/doc"))
        mounts += self.mountinfo(self.runtime / "doc-other")
        with patch.object(Path, "read_text", return_value=mounts), \
                patch.object(smoke_ui.subprocess, "run") as run:
            smoke_ui._cleanup_portal_mount(self.runtime)
        run.assert_not_called()

    def test_unmounts_only_exact_private_portal_and_verifies_removal(self):
        mounted = self.mountinfo()
        with patch.object(Path, "read_text", side_effect=[mounted, mounted, ""]), \
                patch.object(smoke_ui.shutil, "which", return_value="/usr/bin/fusermount3"), \
                patch.object(smoke_ui.subprocess, "run", return_value=
                             subprocess.CompletedProcess([], 0, "")) as run:
            smoke_ui._cleanup_portal_mount(self.runtime)
        run.assert_called_once_with(
            ["/usr/bin/fusermount3", "-u", "--", str(self.runtime / "doc")],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True, timeout=5)
        self.assertFalse((self.runtime / "doc").exists())

    def test_busy_mount_uses_legacy_fusermount_lazy_fallback(self):
        mounted = self.mountinfo()
        with patch.object(Path, "read_text", side_effect=[mounted] * 4 + [""]), \
                patch.object(smoke_ui.shutil, "which", side_effect=[None, "/bin/fusermount"]), \
                patch.object(smoke_ui.subprocess, "run", side_effect=[
                    subprocess.CompletedProcess([], 1, "Device or resource busy"),
                    subprocess.CompletedProcess([], 0, "")]) as run:
            smoke_ui._cleanup_portal_mount(self.runtime)
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["/bin/fusermount", "-u", "--", str(self.runtime / "doc")],
            ["/bin/fusermount", "-u", "-z", "--", str(self.runtime / "doc")]])

    def test_remaining_mount_is_failure_even_if_command_claims_success(self):
        with patch.object(Path, "read_text", return_value=self.mountinfo()), \
                patch.object(smoke_ui.shutil, "which", return_value="/bin/fusermount3"), \
                patch.object(smoke_ui.subprocess, "run", return_value=
                             subprocess.CompletedProcess([], 0, "")) as run:
            with self.assertRaisesRegex(RuntimeError, "Private portal mount remains"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        self.assertEqual(run.call_count, 2)

    def test_unmount_timeout_is_bounded_and_does_not_hide_remaining_mount(self):
        with patch.object(Path, "read_text", return_value=self.mountinfo()), \
                patch.object(smoke_ui.shutil, "which", return_value="/bin/fusermount3"), \
                patch.object(smoke_ui.subprocess, "run", side_effect=
                             subprocess.TimeoutExpired("fusermount3", 5)) as run:
            with self.assertRaisesRegex(RuntimeError, "unmount timed out"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        self.assertEqual(run.call_count, 2)

    def test_wrong_filesystem_at_exact_path_is_never_unmounted(self):
        with patch.object(Path, "read_text", return_value=self.mountinfo(filesystem="tmpfs")), \
                patch.object(smoke_ui.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "Refusing to unmount unexpected tmpfs"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        run.assert_not_called()

    def test_changed_filesystem_before_lazy_retry_is_never_unmounted(self):
        mounted = self.mountinfo()
        with patch.object(Path, "read_text", side_effect=[
                mounted, mounted, mounted, self.mountinfo(filesystem="ext4")]), \
                patch.object(smoke_ui.shutil, "which", return_value="/bin/fusermount3"), \
                patch.object(smoke_ui.subprocess, "run", return_value=
                             subprocess.CompletedProcess([], 1, "busy")) as run:
            with self.assertRaisesRegex(RuntimeError, "Refusing to unmount unexpected ext4"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        self.assertEqual(run.call_count, 1)

    def test_missing_unmount_tool_is_actionable_failure(self):
        with patch.object(Path, "read_text", return_value=self.mountinfo()), \
                patch.object(smoke_ui.shutil, "which", return_value=None), \
                patch.object(smoke_ui.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "fusermount3/fusermount is unavailable"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        run.assert_not_called()

    def test_symlinked_or_shared_runtime_is_rejected(self):
        alias = self.root / "alias"
        alias.symlink_to(self.runtime, target_is_directory=True)
        with patch.object(smoke_ui.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "outside a private runtime"):
                smoke_ui._cleanup_portal_mount(alias)
            self.runtime.chmod(0o755)
            with self.assertRaisesRegex(RuntimeError, "outside a private runtime"):
                smoke_ui._cleanup_portal_mount(self.runtime)
        run.assert_not_called()


class ProcessCleanupTests(unittest.TestCase):
    def test_macos_stops_only_its_app_process(self):
        process = Mock()
        with patch.object(smoke_ui.os, "killpg") as kill_group:
            smoke_ui._stop_macos_app(process)
        process.terminate.assert_called_once_with()
        process.wait.assert_called_once_with(timeout=3)
        process.kill.assert_not_called()
        kill_group.assert_not_called()

    def test_macos_reaps_an_already_exited_app(self):
        process = Mock()
        process.terminate.side_effect = ProcessLookupError
        process.wait.return_value = 0
        smoke_ui._stop_macos_app(process)
        process.wait.assert_called_once_with(timeout=3)
        process.kill.assert_not_called()

    def test_macos_escalates_only_an_unresponsive_app(self):
        process = Mock()
        process.wait.side_effect = [subprocess.TimeoutExpired("UI", 3), -9]
        with patch.object(smoke_ui.os, "killpg") as kill_group:
            smoke_ui._stop_macos_app(process)
        process.terminate.assert_called_once_with()
        process.kill.assert_called_once_with()
        self.assertEqual(process.wait.call_count, 2)
        kill_group.assert_not_called()

    def test_wrapper_exit_still_allows_children_graceful_cleanup(self):
        process = Mock(pid=12345)
        process.poll.return_value = 0
        with patch.object(smoke_ui.os, "killpg", side_effect=[None, None, ProcessLookupError]) as kill, \
                patch.object(smoke_ui.time, "monotonic", side_effect=[0, 0, 0.1]), \
                patch.object(smoke_ui.time, "sleep") as sleep:
            smoke_ui._stop_process_group(process)
        self.assertEqual([call.args for call in kill.call_args_list], [
            (12345, signal.SIGTERM), (12345, 0), (12345, 0)])
        sleep.assert_called_once_with(0.05)
        process.wait.assert_called_once_with(timeout=3)

    def test_surviving_children_are_killed_after_grace_period(self):
        process = Mock(pid=12345)
        process.poll.return_value = 0
        with patch.object(smoke_ui.os, "killpg") as kill, \
                patch.object(smoke_ui.time, "monotonic", side_effect=[0, 0, 3]), \
                patch.object(smoke_ui.time, "sleep"):
            smoke_ui._stop_process_group(process)
        self.assertEqual([call.args for call in kill.call_args_list], [
            (12345, signal.SIGTERM), (12345, 0), (12345, signal.SIGKILL)])
        process.wait.assert_called_once_with(timeout=3)


class StartupCleanupTests(unittest.TestCase):
    def test_macos_cleanup_never_manages_groups_or_portals(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory).resolve()
            env = dict(os.environ, HOME=str(home))
            database = home / "Library/Application Support/swictation/metrics.db"
            process = Mock(pid=12345)
            process.wait.side_effect = subprocess.TimeoutExpired("UI", 8)

            def start(command, **kwargs):
                kwargs["stdout"].write(f'Metrics database path: "{database}"\n'.encode())
                kwargs["stdout"].flush()
                return process

            with patch.object(smoke_ui.subprocess, "Popen", side_effect=start), \
                    patch.object(smoke_ui, "_stop_macos_app") as stop, \
                    patch.object(smoke_ui, "_stop_process_group") as group, \
                    patch.object(smoke_ui, "_cleanup_portal_mount") as portal, \
                    redirect_stdout(io.StringIO()):
                errors = smoke_ui.check_ui_startup(home / "release", home, env,
                                                   "aarch64-apple-darwin")
            self.assertEqual(errors, [])
            stop.assert_called_once_with(process)
            group.assert_not_called()
            portal.assert_not_called()

    def test_success_and_failure_clean_only_helper_created_runtime(self):
        for alive in (True, False):
            with self.subTest(alive=alive), tempfile.TemporaryDirectory() as directory:
                home = Path(directory).resolve()
                env = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(home / "data"),
                           XDG_RUNTIME_DIR="/run/user/1000")
                expected_database = home / "data/swictation/metrics.db"
                process = Mock(pid=12345)
                if alive:
                    process.wait.side_effect = subprocess.TimeoutExpired("UI", 8)
                else:
                    process.wait.return_value = 1

                def start(command, **kwargs):
                    kwargs["stdout"].write(f'Metrics database path: "{expected_database}"\n'.encode())
                    kwargs["stdout"].flush()
                    return process

                with patch.object(smoke_ui.subprocess, "Popen", side_effect=start) as popen, \
                        patch.object(smoke_ui, "_stop_process_group") as stop, \
                        patch.object(smoke_ui, "_cleanup_portal_mount") as cleanup, \
                        redirect_stdout(io.StringIO()):
                    result = smoke_ui.check_ui_startup(home / "release", home, env,
                                                       "x86_64-unknown-linux-gnu")
                runtime = home / "tmp/ui-runtime"
                self.assertEqual(runtime.stat().st_mode & 0o777, 0o700)
                self.assertEqual(popen.call_args.kwargs["env"]["XDG_RUNTIME_DIR"], str(runtime))
                self.assertEqual(env["XDG_RUNTIME_DIR"], "/run/user/1000")
                stop.assert_called_once_with(process)
                cleanup.assert_called_once_with(runtime)
                self.assertEqual(bool(result), not alive)


if __name__ == "__main__":
    unittest.main()
