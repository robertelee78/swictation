#!/usr/bin/env bash
# CI uses three Apple release variables and three secrets (see README.md). Local signing can reuse an existing
# identity without importing a certificate or changing keychain search lists:
#   APPLE_DEVELOPER_ID_APPLICATION='Developer ID Application: ... (TEAMID)'
#   APPLE_TEAM_ID=TEAMID bash sign-macos.sh PAYLOAD --local-keychain \
#     --keychain-profile PROFILE [--keychain /absolute/signing.keychain-db]
# APPLE_NOTARY_KEYCHAIN_PROFILE is the environment equivalent of the profile
# argument. Without a profile, both modes require the notary API key and its two public identifiers.
set +x
set -euo pipefail
fail() { printf 'release signing: %s\n' "$1" >&2; exit 1; }
if [[ $# -lt 1 || ! -d "$1" ]]; then
  echo 'usage: sign-macos.sh STAGED_PAYLOAD [--local-keychain] [--keychain PATH] [--keychain-profile NAME]' >&2
  exit 1
fi
payload=$(cd "$1" && pwd -P)
shift
local_keychain=false
signing_keychain=''
notary_profile=${APPLE_NOTARY_KEYCHAIN_PROFILE:-}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --local-keychain) local_keychain=true; shift ;;
    --keychain)
      [[ $# -ge 2 && -n "$2" ]] || fail '--keychain requires an existing absolute keychain path'
      signing_keychain=$2; shift 2 ;;
    --keychain-profile)
      [[ $# -ge 2 && -n "$2" ]] || fail '--keychain-profile requires a profile name'
      notary_profile=$2; shift 2 ;;
    *) fail 'unknown signing argument' ;;
  esac
done
[[ -z "$signing_keychain" || "$local_keychain" == true ]] || fail '--keychain requires --local-keychain'
if [[ -n "$signing_keychain" ]]; then
  [[ "$signing_keychain" == /* && -f "$signing_keychain" && ! -L "$signing_keychain" ]] \
    || fail 'signing keychain must be an existing absolute regular file'
fi
for name in APPLE_DEVELOPER_ID_APPLICATION APPLE_TEAM_ID; do
  [[ -n "${!name:-}" ]] || fail "missing $name"
done
signing_identity=$APPLE_DEVELOPER_ID_APPLICATION
[[ "$APPLE_TEAM_ID" =~ ^[A-Z0-9]{10}$ ]] || { echo 'invalid Apple team ID' >&2; exit 1; }
[[ "$signing_identity" == "Developer ID Application: "*" ($APPLE_TEAM_ID)" ]] \
  || { echo 'signing identity must belong to the pinned Developer ID team' >&2; exit 1; }
[[ "$signing_identity" != *$'\n'* && ${#signing_identity} -le 255 ]] || fail 'invalid signing identity'
p12_base64=''
p12_password=''
if [[ "$local_keychain" != true ]]; then
  for name in APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64 APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD; do
    [[ -n "${!name:-}" ]] || fail "missing $name"
  done
  p12_base64=$APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64
  p12_password=$APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD
fi
notary_key_base64=''
notary_key_id=''
notary_issuer_id=''
if [[ -n "$notary_profile" ]]; then
  [[ "$notary_profile" != *$'\n'* && ${#notary_profile} -le 255 ]] || fail 'invalid notary keychain profile'
else
  for name in APPLE_NOTARY_KEY_P8_BASE64 APPLE_NOTARY_KEY_ID APPLE_NOTARY_ISSUER_ID; do
    [[ -n "${!name:-}" ]] || fail "missing $name"
  done
  notary_key_base64=$APPLE_NOTARY_KEY_P8_BASE64
  notary_key_id=$APPLE_NOTARY_KEY_ID
  notary_issuer_id=$APPLE_NOTARY_ISSUER_ID
  [[ "$notary_key_id" =~ ^[A-Z0-9]{10}$ ]] || fail 'invalid notary API key ID'
  [[ "$notary_issuer_id" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]] \
    || fail 'invalid notary API issuer ID'
fi
unset APPLE_DEVELOPER_ID_APPLICATION APPLE_DEVELOPER_ID_APPLICATION_P12_BASE64 \
  APPLE_DEVELOPER_ID_APPLICATION_P12_PASSWORD APPLE_NOTARY_KEY_P8_BASE64 \
  APPLE_NOTARY_KEY_ID APPLE_NOTARY_ISSUER_ID
repository=$(cd "$(dirname "$0")/../.." && pwd -P)
umask 077
temporary=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/swictation-sign.XXXXXXXX")
created_keychain=''
cleanup() {
  if [[ -n "$created_keychain" ]]; then security delete-keychain "$created_keychain" >/dev/null 2>&1 || true; fi
  rm -rf "$temporary"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
if [[ "$local_keychain" != true ]]; then
  created_keychain="$temporary/release.keychain-db"
  signing_keychain=$created_keychain
  keychain_password=$(openssl rand -hex 32)
  printf '%s' "$p12_base64" | base64 --decode > "$temporary/certificate.p12"
  [[ -s "$temporary/certificate.p12" ]] || fail 'empty signing certificate material'
  unset p12_base64
  security create-keychain -p "$keychain_password" "$created_keychain"
  security set-keychain-settings -lut 21600 "$created_keychain"
  security unlock-keychain -p "$keychain_password" "$created_keychain"
  security import "$temporary/certificate.p12" -P "$p12_password" \
    -t cert -f pkcs12 -k "$created_keychain" -T /usr/bin/codesign
  # Match the isolated imported identity before granting access to its key.
  identities=$(security find-identity -v -p codesigning "$created_keychain")
  [[ $(grep -Fc -- "\"$signing_identity\"" <<< "$identities") -eq 1 ]] \
    || fail 'temporary keychain does not contain the intended signing identity'
  [[ $(grep -Ec '^[[:space:]]*1 valid identities found$' <<< "$identities") -eq 1 ]] \
    || fail 'temporary keychain contains an ambiguous signing identity set'
  # Imported PKCS#12 keys do not consistently match security's -s attribute
  # filter across macOS versions. This keychain contains only our imported key.
  security set-key-partition-list -S apple-tool:,apple:,codesign: \
    -k "$keychain_password" "$created_keychain" >/dev/null
  unset p12_password keychain_password
fi
notary_auth=()
if [[ -n "$notary_profile" ]]; then
  notary_auth=(--keychain-profile "$notary_profile")
else
  notary_key="$temporary/AuthKey_$notary_key_id.p8"
  printf '%s' "$notary_key_base64" | base64 --decode > "$notary_key"
  [[ -s "$notary_key" ]] || fail 'empty notary API key material'
  unset notary_key_base64
  notary_auth=(--key "$notary_key" --key-id "$notary_key_id" --issuer "$notary_issuer_id")
fi
signing_args=(--sign "$signing_identity")
if [[ -n "$signing_keychain" ]]; then signing_args+=(--keychain "$signing_keychain"); fi
sign() {
  codesign --force "${signing_args[@]}" --options runtime --timestamp "$@"
}
# Libraries first, then executables, then the enclosing bundle. The daemon's
# stable identifier preserves microphone/accessibility identity across updates.
while IFS= read -r -d '' library; do sign "$library"; done < <(find "$payload" -type f -name '*.dylib' -print0)
sign --identifier com.swictation.cli "$payload/bin/swictation"
sign --identifier com.swictation.daemon \
  --entitlements "$repository/rust-crates/swictation-daemon/entitlements/daemon.entitlements" \
  "$payload/bin/swictation-daemon"
sign --identifier com.swictation.ui \
  --entitlements "$repository/tauri-ui/src-tauri/entitlements/ui.entitlements" "$payload/bin/swictation-ui"
for component in daemon ui; do
  if [[ "$component" == daemon ]]; then
    app="$payload/share/SwictationDaemon.app"
    entitlements="$repository/rust-crates/swictation-daemon/entitlements/daemon.entitlements"
  else
    app="$payload/share/Swictation.app"
    entitlements="$repository/tauri-ui/src-tauri/entitlements/ui.entitlements"
  fi
  test -d "$app"
  sign --identifier "com.swictation.$component" --entitlements "$entitlements" \
    "$app/Contents/MacOS/swictation-$component"
  sign --identifier "com.swictation.$component" --entitlements "$entitlements" "$app"
done
ditto -c -k --keepParent "$payload" "$temporary/submission.zip"
xcrun notarytool submit "$temporary/submission.zip" "${notary_auth[@]}" --wait \
  --timeout 30m --output-format json > "$temporary/notarization.json"
python3 - "$temporary/notarization.json" <<'PY'
import json, sys
result = json.load(open(sys.argv[1]))
if result.get("status") != "Accepted":
    raise SystemExit(f"notarization was not accepted: {result.get('status', 'missing status')}")
print(f"Apple notarization accepted: {result['id']}")
PY
for app in "$payload/share/SwictationDaemon.app" "$payload/share/Swictation.app"; do
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  codesign --verify --deep --strict --all-architectures "$app"
done
for binary in "$payload/bin/swictation" "$payload/bin/swictation-daemon" "$payload/bin/swictation-ui"; do
  codesign --verify --strict --all-architectures --check-notarization \
    --test-requirement '=notarized' "$binary"
  signing=$(codesign --display --verbose=4 "$binary" 2>&1)
  printf '%s\n' "$signing" | grep -qx "TeamIdentifier=$APPLE_TEAM_ID"
done
