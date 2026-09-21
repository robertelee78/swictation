"""Signing command contracts using fake Apple tools and dummy credential bytes."""
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("sign-macos.sh")
TEAM = "ABCDEFGHIJ"
SECRET_NAMES = ["APPLE_DEVELOPER_ID_APPLICATION", "APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64",
                "APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD", "APPLE_NOTARY_KEY_P8_BASE64",
                "APPLE_NOTARY_KEY_ID", "APPLE_NOTARY_ISSUER_ID"]


class SigningContracts(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="swictation-signing-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.payload = self.root / "payload"
        for name in ("bin/swictation", "bin/swictation-daemon", "bin/swictation-ui", "lib/runtime.dylib",
                     "share/SwictationDaemon.app/Contents/MacOS/swictation-daemon",
                     "share/Swictation.app/Contents/MacOS/swictation-ui"):
            path = self.payload / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"unsigned fixture")
        commands = self.root / "commands"
        commands.mkdir()
        self.log = self.root / "commands.jsonl"
        for tool in ("security", "codesign", "ditto", "xcrun"):
            command = commands / tool
            command.write_text(f"""#!{sys.executable}
import json, os, pathlib, sys
tool = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
record = {{'tool': tool, 'args': args, 'leaked_environment': [key for key in {SECRET_NAMES!r} if key in os.environ]}}
with open(os.environ['MOCK_SIGNING_LOG'], 'a') as log:
    log.write(json.dumps(record) + '\\n')
if tool == 'codesign' and '--display' in args:
    print('TeamIdentifier={TEAM}', file=sys.stderr)
if tool == 'codesign' and '--sign' in args and os.environ.get('MOCK_SIGNING_FAIL'):
    sys.exit('Signing failure fixture')
if tool == 'security' and args == ['list-keychains', '-d', 'user']:
    print('    "/existing/login.keychain-db"')
    print('    "/existing/System.keychain"')
if tool == 'security' and args[0] == 'find-identity':
    print('  1) ' + 'A' * 40 + ' "Developer ID Application: Fixture ({TEAM})"')
    print('     1 valid identities found')
if tool == 'ditto':
    pathlib.Path(args[-1]).write_bytes(b'notary submission fixture')
if tool == 'xcrun' and args[:2] == ['notarytool', 'submit']:
    print(json.dumps({{'status': 'Accepted', 'id': 'mock-notary-submission'}}))
""")
            command.chmod(0o755)
        self.env = {key: value for key, value in os.environ.items() if not key.startswith("APPLE_")}
        self.env.update(PATH=f"{commands}:{os.environ['PATH']}", RUNNER_TEMP=str(self.root),
                        MOCK_SIGNING_LOG=str(self.log), APPLE_TEAM_ID=TEAM,
                        APPLE_DEVELOPER_ID_APPLICATION=f"Developer ID Application: Fixture ({TEAM})")

    def run_signer(self, *arguments, success=True):
        result = subprocess.run(["bash", str(SCRIPT), str(self.payload), *arguments], env=self.env,
                                text=True, capture_output=True)
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0)
        records = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        self.assertTrue(all(not record["leaked_environment"] for record in records))
        self.assertFalse(any(record["tool"] == "security" and record["args"][0] == "default-keychain"
                             for record in records))
        self.assertEqual(list(self.root.glob("swictation-sign.*")), [])
        return records

    def configure_ci_credentials(self):
        self.env.update(APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64=base64.b64encode(b"dummy p12").decode(),
                        APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD="dummy-password",
                        APPLE_NOTARY_KEY_P8_BASE64=base64.b64encode(b"dummy p8").decode(),
                        APPLE_NOTARY_KEY_ID="1234567890",
                        APPLE_NOTARY_ISSUER_ID="11111111-2222-3333-4444-555555555555")

    def assert_search_list_restored(self, records):
        lists = [record["args"] for record in records if record["tool"] == "security"
                 and record["args"][0] == "list-keychains"]
        self.assertEqual(len(lists), 3)
        self.assertEqual(lists[0], ["list-keychains", "-d", "user"])
        self.assertEqual(lists[1][:4], ["list-keychains", "-d", "user", "-s"])
        self.assertTrue(lists[1][4].endswith("/release.keychain-db"))
        existing = ["/existing/login.keychain-db", "/existing/System.keychain"]
        self.assertEqual(lists[1][5:], existing)
        self.assertEqual(lists[2], ["list-keychains", "-d", "user", "-s", *existing])

    def test_ci_uses_api_key_secrets_and_cleans_temporary_credentials(self):
        self.configure_ci_credentials()
        records = self.run_signer()
        self.assert_search_list_restored(records)
        security = [record["args"][0] for record in records if record["tool"] == "security"]
        self.assertIn("import", security)
        self.assertIn("find-identity", security)
        self.assertIn("delete-keychain", security)
        partition = next(record["args"] for record in records if record["tool"] == "security"
                         and record["args"][0] == "set-key-partition-list")
        self.assertNotIn("-s", partition)
        self.assertTrue(partition[-1].endswith("/release.keychain-db"))
        submission = next(record["args"] for record in records if record["tool"] == "xcrun"
                          and record["args"][:2] == ["notarytool", "submit"])
        self.assertIn("--key", submission)
        self.assertIn("--issuer", submission)
        self.assertNotIn("--apple-id", submission)
        self.assertNotIn("--password", submission)
        stapled = [record["args"][-1] for record in records if record["tool"] == "xcrun"
                   and record["args"][:2] == ["stapler", "staple"]]
        self.assertEqual({Path(path).name for path in stapled}, {"Swictation.app", "SwictationDaemon.app"})

    def test_ci_restores_search_list_and_deletes_keychain_after_signing_failure(self):
        self.configure_ci_credentials()
        self.env["MOCK_SIGNING_FAIL"] = "1"
        records = self.run_signer(success=False)
        self.assert_search_list_restored(records)
        self.assertTrue(any(record["tool"] == "security" and record["args"][0] == "delete-keychain"
                            for record in records))
        self.assertFalse(any(record["tool"] == "xcrun" for record in records))

    def test_local_keychain_profile_never_imports_or_deletes_a_keychain(self):
        records = self.run_signer("--local-keychain", "--keychain-profile", "existing-notary-profile")
        self.assertFalse(any(record["tool"] == "security" for record in records))
        submission = next(record["args"] for record in records if record["tool"] == "xcrun"
                          and record["args"][:2] == ["notarytool", "submit"])
        self.assertIn("--keychain-profile", submission)
        self.assertIn("existing-notary-profile", submission)
        self.assertNotIn("--key", submission)

    def test_local_explicit_signing_keychain_and_environment_notary_profile(self):
        keychain = self.root / "existing.keychain-db"
        keychain.write_bytes(b"existing keychain fixture")
        self.env["APPLE_NOTARY_KEYCHAIN_PROFILE"] = "profile-from-environment"
        records = self.run_signer("--local-keychain", "--keychain", str(keychain))
        self.assertEqual(keychain.read_bytes(), b"existing keychain fixture")
        self.assertFalse(any(record["tool"] == "security" for record in records))
        signing = [record["args"] for record in records if record["tool"] == "codesign" and "--sign" in record["args"]]
        self.assertTrue(signing)
        self.assertTrue(all(args[args.index("--keychain") + 1] == str(keychain) for args in signing))

    def test_missing_notarization_credentials_fail_before_any_signing(self):
        records = self.run_signer("--local-keychain", success=False)
        self.assertEqual(records, [])


if __name__ == "__main__":
    unittest.main()
