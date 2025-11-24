# macOS Testing Checklist and Test Plan

Comprehensive test plan for Swictation macOS support (v0.7.0+).

---

## Test Environments

### Hardware Requirements
- [ ] **macOS 14 Sonoma** (M1/M2/M3) - Minimum supported version
- [ ] **macOS 15 Sequoia** (M1/M2/M3/M4) - Latest version
- [ ] Various RAM configurations:
  - [ ] 8GB (CPU fallback expected)
  - [ ] 16GB (0.6B GPU model expected)
  - [ ] 32GB+ (1.1B GPU model expected)

### Software Prerequisites
- [ ] Node.js 18+ installed
- [ ] npm configured with global prefix
- [ ] Clean user account (no prior Swictation installation)

---

## 1. Installation Tests

### 1.1 Platform Detection
- [ ] Install succeeds on Apple Silicon (M1/M2/M3/M4)
- [ ] Install BLOCKS on Intel Mac with clear error message
- [ ] Install BLOCKS on macOS 13 or earlier with version error
- [ ] `uname -m` shows `arm64` during installation

### 1.2 npm Installation
```bash
npm install -g swictation --foreground-scripts
```
- [ ] Installation completes without errors
- [ ] postinstall.js detects macOS platform
- [ ] Unified memory detection runs and shows correct values
- [ ] Model recommendation matches RAM (8GB→CPU, 16GB→0.6B, 32GB+→1.1B)

### 1.3 Binary Downloads
Check: `~/.npm-global/lib/node_modules/swictation/lib/native/`
- [ ] `swictation-daemon-macos` exists and is executable
- [ ] `swictation-ui-macos` exists and is executable (if Tauri built)
- [ ] `libonnxruntime.dylib` exists (~30-50MB)
- [ ] No CUDA libraries downloaded (Linux-specific)

### 1.4 Model Downloads
Check: `~/.local/share/swictation/models/`
- [ ] Recommended model downloaded (0.6B or 1.1B)
- [ ] FP16 model variant selected (`.fp16.onnx` files)
- [ ] Encoder and decoder models present
- [ ] Total model size reasonable (0.6B ~600MB, 1.1B ~1.1GB)

### 1.5 Service Installation
Check: `~/Library/LaunchAgents/`
- [ ] `com.swictation.daemon.plist` created
- [ ] `com.swictation.ui.plist` created (if Tauri enabled)
- [ ] Plist files valid: `plutil -lint ~/Library/LaunchAgents/com.swictation.*.plist`
- [ ] ORT_DYLIB_PATH environment variable set in plists

### 1.6 Configuration
Check: `~/.config/swictation/`
- [ ] `config.toml` created
- [ ] Default hotkey set to Cmd+Shift+D
- [ ] VAD threshold set to 0.25
- [ ] Silence duration set to 0.8s
- [ ] Log directory configured

---

## 2. Accessibility Permissions

### 2.1 Initial Permission Request
- [ ] First daemon launch prompts for Accessibility permissions
- [ ] Error message clear if permissions not granted
- [ ] Instructions point to correct System Settings location

### 2.2 Granting Permissions
Navigate: **System Settings → Privacy & Security → Accessibility**
- [ ] Can find swictation-daemon-macos binary at correct path
- [ ] Checkbox appears after adding binary
- [ ] Enabling checkbox works without errors
- [ ] Lock icon prevents unauthorized changes

### 2.3 Permission Verification
```bash
swictation start
swictation status
```
- [ ] Daemon starts successfully after permissions granted
- [ ] Status shows "Active" and "Connected"
- [ ] No permission errors in logs: `tail -f ~/Library/Logs/swictation/daemon-error.log`

---

## 3. Text Injection Tests

### 3.1 Basic Text Injection - TextEdit
1. Open TextEdit (new document)
2. Press `Cmd+Shift+D` to start recording
3. Say: "Hello world period"
4. Wait 1 second for silence detection
5. Press `Cmd+Shift+D` to stop

**Verify:**
- [ ] Text appears: "Hello world."
- [ ] Capitalization correct (first letter uppercase)
- [ ] Punctuation correct (period added)
- [ ] No duplicate text
- [ ] Cursor positioned after text

### 3.2 Text Injection - Notes App
Repeat test 3.1 in Notes.app
- [ ] Text injection works correctly
- [ ] No permission errors
- [ ] Performance similar to TextEdit

### 3.3 Text Injection - Terminal
Open Terminal and type partial command, then dictate:
```bash
echo "hello # then dictate: "world"
```
- [ ] Text injection works in Terminal
- [ ] No shell interpretation issues
- [ ] Special characters handled correctly

### 3.4 Text Injection - Safari/Chrome
Test in browser text field (e.g., Google search)
- [ ] Text injection works in web browser
- [ ] JavaScript events triggered correctly
- [ ] Form submission behavior normal

### 3.5 Unicode Characters
Dictate text containing Unicode:
- [ ] Emoji: "smiley face emoji" → 😊 (if supported)
- [ ] Accented characters: "café", "naïve"
- [ ] Mathematical symbols (if in vocabulary)
- [ ] Non-ASCII characters display correctly

### 3.6 Multi-line Text
Dictate with multiple sentences:
- [ ] Multiple sentences work
- [ ] Newlines NOT inserted automatically (expected behavior)
- [ ] Paragraph breaks require manual Enter key

---

## 4. Hotkey Tests

### 4.1 Default Hotkey
- [ ] `Cmd+Shift+D` starts recording
- [ ] `Cmd+Shift+D` again stops recording
- [ ] Hotkey works in all applications
- [ ] No conflicts with system shortcuts
- [ ] Hotkey works after app switch

### 4.2 Hotkey Conflicts
Test with apps that use Cmd+Shift+D:
- [ ] Swictation hotkey takes priority
- [ ] Check System Settings → Keyboard → Keyboard Shortcuts for conflicts
- [ ] Document any conflicting apps

### 4.3 Custom Hotkeys (Future Feature)
*Note: Custom hotkeys not yet supported on macOS as of v0.7.0*
- [ ] Config file has placeholder for future custom hotkey support
- [ ] Documentation notes this limitation

---

## 5. GPU Acceleration Tests

### 5.1 CoreML Provider Loading
Check logs: `grep -i "coreml\|gpu\|metal" ~/Library/Logs/swictation/daemon.log`
- [ ] "Enabling CoreML execution provider" message appears
- [ ] No errors loading CoreML provider
- [ ] MLProgram format mentioned
- [ ] Compute units set to "All" (CPU+GPU+ANE)

### 5.2 Unified Memory Detection
```bash
# Check system memory
sysctl hw.memsize
```
Compare with postinstall output:
- [ ] Total system memory detected correctly
- [ ] GPU share calculated as 35% of total RAM
- [ ] Memory values match: System Settings → General → About → Memory

### 5.3 GPU Memory Usage During Inference
Open Activity Monitor → GPU tab while recording:
- [ ] GPU usage spikes during transcription
- [ ] Memory usage increases (0.6B ~1.2GB, 1.1B ~3.5GB)
- [ ] GPU usage returns to baseline after transcription
- [ ] No GPU memory leaks over multiple sessions

### 5.4 Model Selection Verification
Check config.toml or logs for model selection:
- [ ] **8GB Mac**: CPU fallback (no GPU model loaded)
- [ ] **16GB Mac**: 0.6B GPU model loaded
- [ ] **32GB+ Mac**: 1.1B GPU model loaded
- [ ] FP16 models used (not INT8 or FP32)

### 5.5 Performance Metrics
Measure latency with stopwatch:
- [ ] VAD detection: <100ms from silence to transcription start
- [ ] STT inference: 150-300ms for typical phrase (M1 GPU)
- [ ] Total latency: <1 second from speech end to text appearance
- [ ] Performance comparable to documentation claims

---

## 6. Secretary Mode Tests

### 6.1 Punctuation Commands
Test each punctuation command:
- [ ] "period" → "."
- [ ] "comma" → ","
- [ ] "exclamation mark" → "!"
- [ ] "question mark" → "?"
- [ ] "semicolon" → ";"
- [ ] "colon" → ":"
- [ ] "apostrophe" → "'"
- [ ] "hyphen" → "-"
- [ ] "dash" → "—"

### 6.2 Quote Commands
- [ ] "open quote" → " (opening smart quote)
- [ ] "close quote" → " (closing smart quote)
- [ ] "quote ... end quote" → "..." (paired quotes)

### 6.3 Number Commands
- [ ] "number forty two" → "42"
- [ ] "number one thousand" → "1000"
- [ ] "dollars fifty" → "$50"
- [ ] "percent twenty" → "20%"

### 6.4 Formatting Commands
- [ ] "new line" → newline character
- [ ] "new paragraph" → double newline
- [ ] "cap next word" → Next Word (capitalized)
- [ ] "all caps" → ALL CAPS MODE
- [ ] "no caps" → no caps mode

### 6.5 Bracket/Symbol Commands
- [ ] "open paren" → "("
- [ ] "close paren" → ")"
- [ ] "open bracket" → "["
- [ ] "close bracket" → "]"
- [ ] "open brace" → "{"
- [ ] "close brace" → "}"

### 6.6 Programming Symbols
- [ ] "slash" → "/"
- [ ] "backslash" → "\\"
- [ ] "asterisk" → "*"
- [ ] "plus" → "+"
- [ ] "equals" → "="
- [ ] "ampersand" → "&"
- [ ] "at sign" → "@"
- [ ] "hash" → "#"

---

## 7. Service Management Tests

### 7.1 Start/Stop/Restart
```bash
swictation start
swictation stop
swictation restart
```
- [ ] `start` launches daemon successfully
- [ ] `stop` terminates daemon cleanly
- [ ] `restart` stops and starts without errors
- [ ] No zombie processes left behind: `ps aux | grep swictation`

### 7.2 Status Command
```bash
swictation status
```
- [ ] Shows daemon status (Active/Inactive)
- [ ] Shows socket status (Connected/Disconnected)
- [ ] Shows PID when running
- [ ] Shows uptime
- [ ] Accurate status reporting

### 7.3 LaunchAgent Management
```bash
launchctl list | grep swictation
```
- [ ] Daemon service listed when running
- [ ] UI service listed (if Tauri enabled)
- [ ] Service PID matches `swictation status` output

### 7.4 Auto-start on Login
1. Start services: `swictation start`
2. Verify auto-start enabled: `launchctl list | grep swictation`
3. Log out and log back in
4. Check if daemon running: `swictation status`

**Verify:**
- [ ] Daemon auto-starts on login
- [ ] No manual start needed
- [ ] Hotkey works immediately after login

### 7.5 System Reboot Test
1. Start services: `swictation start`
2. Reboot Mac
3. Log in
4. Check status: `swictation status`

**Verify:**
- [ ] Services survive reboot
- [ ] Auto-start works after reboot
- [ ] No configuration lost

### 7.6 Disable Auto-start
```bash
launchctl unload ~/Library/LaunchAgents/com.swictation.daemon.plist
```
- [ ] Daemon stops
- [ ] Does not restart on login
- [ ] Can re-enable with `launchctl load`

---

## 8. Tauri UI Tests

### 8.1 UI Launch
```bash
# UI should auto-start with daemon, or launch manually
open ~/.npm-global/lib/node_modules/swictation/lib/native/swictation-ui-macos
```
- [ ] UI window appears
- [ ] System tray icon visible
- [ ] No launch errors
- [ ] UI responsive

### 8.2 Metrics Display
- [ ] Real-time audio level meter works
- [ ] VAD probability updates during speech
- [ ] Transcription latency displayed
- [ ] CPU usage shown
- [ ] GPU usage shown (if Metal API implemented)
- [ ] Memory usage displayed

### 8.3 Session History
- [ ] Previous transcriptions listed
- [ ] Timestamps accurate
- [ ] Can scroll through history
- [ ] Search functionality works (if implemented)
- [ ] Export history works (if implemented)

### 8.4 Settings Panel
- [ ] Can view current configuration
- [ ] VAD threshold adjustable (slider or input)
- [ ] Silence duration adjustable
- [ ] Model selection dropdown (if implemented)
- [ ] Settings persist after restart

### 8.5 Corrections Learning UI
- [ ] Can view learned corrections
- [ ] Can add new correction patterns
- [ ] Can delete corrections
- [ ] "Learn from edit" button works
- [ ] Phonetic matching threshold adjustable

---

## 9. Corrections Learning Tests

### 9.1 Basic Correction Learning
1. Dictate: "arkon"
2. Edit to: "Archon"
3. Click "Learn" button (or equivalent)
4. Dictate "arkon" again

**Verify:**
- [ ] Second dictation produces "Archon" automatically
- [ ] Correction saved to `~/.config/swictation/corrections.toml`
- [ ] No daemon restart needed (hot-reload works)

### 9.2 Phonetic Fuzzy Matching
Add correction: "arkon" → "Archon" (threshold 0.3)
Test variations:
- [ ] "archon" → "Archon"
- [ ] "arkohn" → "Archon"
- [ ] "arckon" → "Archon"
- [ ] Dissimilar words NOT matched (e.g., "bacon" stays "bacon")

### 9.3 Case Intelligence
- [ ] Force uppercase: "api" → "API"
- [ ] Force title case: "iphone" → "iPhone"
- [ ] Preserve case: "GitHub" → "GitHub" (mixed case)

### 9.4 Multi-word Corrections
- [ ] "postgres" → "PostgreSQL"
- [ ] "kube control" → "kubectl"
- [ ] "type script" → "TypeScript"

### 9.5 Usage Tracking
Check corrections.toml for usage stats:
- [ ] Usage count increments on each match
- [ ] Last used timestamp updates
- [ ] Can sort by most-used corrections (UI feature)

---

## 10. Performance Tests

### 10.1 Latency Measurements
Use stopwatch to measure:
- [ ] **Cold start**: First transcription after daemon launch
  - Target: <2 seconds (includes model loading)
- [ ] **Warm inference**: Subsequent transcriptions
  - Target: <1 second (M1 16GB with 0.6B GPU)
  - Target: <1.5 seconds (M1 32GB with 1.1B GPU)
- [ ] **VAD trigger time**: Silence detection accuracy
  - Should trigger after 0.8s silence (default config)

### 10.2 Resource Usage Baseline
Monitor with Activity Monitor:
- [ ] **Idle daemon**: <100MB RAM, <1% CPU
- [ ] **During inference**:
  - 0.6B model: ~1.5GB RAM, 50-80% GPU
  - 1.1B model: ~3.5GB RAM, 60-90% GPU
- [ ] **After inference**: Resources released within 5 seconds

### 10.3 Sustained Usage Test
Dictate continuously for 10 minutes:
- [ ] No performance degradation
- [ ] No memory leaks (RAM stable)
- [ ] No GPU memory leaks (Activity Monitor)
- [ ] Daemon remains responsive

### 10.4 Stress Test - Rapid Dictation
Speak very quickly with minimal pauses (0.8s) for 5 minutes:
- [ ] All phrases transcribed correctly
- [ ] No dropped audio
- [ ] No buffer overflows
- [ ] Latency remains consistent

### 10.5 Battery Impact (Laptops)
Run on battery power:
- [ ] Measure battery drain during 1 hour of mixed usage
- [ ] Compare to idle battery drain
- [ ] Document energy impact in Activity Monitor

---

## 11. Error Handling Tests

### 11.1 Missing Accessibility Permissions
Remove permissions in System Settings, then restart daemon:
- [ ] Clear error message displayed
- [ ] Instructions for granting permissions shown
- [ ] Daemon exits gracefully
- [ ] No crashes

### 11.2 Missing Models
Delete model files: `rm -rf ~/.local/share/swictation/models/`
Restart daemon:
- [ ] Error message indicates missing models
- [ ] Daemon suggests re-running `swictation download-models`
- [ ] No crashes

### 11.3 Corrupted Configuration
Corrupt config.toml with invalid TOML syntax:
- [ ] Daemon detects invalid config
- [ ] Error message shows line number
- [ ] Daemon falls back to defaults or exits gracefully

### 11.4 Missing ONNX Runtime Library
Rename libonnxruntime.dylib:
- [ ] Daemon detects missing library
- [ ] Error message shows expected path
- [ ] Suggests reinstalling package

### 11.5 Disk Space Exhaustion
Fill disk to near capacity:
- [ ] Daemon handles disk full error gracefully
- [ ] No data corruption
- [ ] Clear error message about disk space

---

## 12. Integration Tests

### 12.1 Multiple Applications Simultaneously
Open TextEdit, Notes, Terminal, Safari:
- [ ] Hotkey works in all apps
- [ ] Text injection works in all apps
- [ ] No app-specific bugs
- [ ] App switching doesn't break functionality

### 12.2 Long-running Session
Run daemon for 24 hours continuously:
- [ ] Daemon remains stable
- [ ] No memory leaks
- [ ] Hotkey still responsive
- [ ] Logs don't grow excessively

### 12.3 Model Switching (If Implemented)
Change model in config: 0.6B ↔ 1.1B
- [ ] Daemon detects config change
- [ ] Hot-reloads new model (or requires restart)
- [ ] No crashes during switch
- [ ] Performance changes as expected

### 12.4 Network Interruption
Disconnect network (if using cloud features):
- [ ] Local inference still works (offline mode)
- [ ] No unexpected errors
- [ ] Graceful degradation

---

## 13. Regression Tests (Linux Compatibility)

**CRITICAL**: Ensure macOS changes didn't break Linux functionality

### 13.1 Linux Installation
On Ubuntu 24.04 with NVIDIA GPU:
```bash
npm install -g swictation --foreground-scripts
```
- [ ] Installation completes without errors
- [ ] CUDA libraries downloaded (NOT CoreML)
- [ ] systemd services installed (NOT launchd)
- [ ] Linux-specific code paths executed

### 13.2 Linux Functionality
- [ ] Daemon starts on Linux: `systemctl --user start swictation-daemon`
- [ ] Hotkey works: `Super+Shift+D`
- [ ] Text injection works with xdotool/wtype/ydotool
- [ ] GPU acceleration works (nvidia-smi shows usage)
- [ ] Performance unchanged from pre-macOS versions

### 13.3 Cross-Platform Configuration
- [ ] Same config.toml format works on both platforms
- [ ] Corrections learned on macOS work on Linux (and vice versa)
- [ ] Models portable between platforms (if same arch)

---

## 14. Documentation Verification

### 14.1 README.md Accuracy
- [ ] macOS listed as supported platform
- [ ] Prerequisites accurate
- [ ] Installation steps work as documented
- [ ] Hotkey documentation correct (Cmd+Shift+D)
- [ ] GPU support table accurate

### 14.2 docs/macos-setup.md Completeness
- [ ] All installation steps accurate
- [ ] Accessibility permissions section correct
- [ ] Service management commands work
- [ ] Troubleshooting section helpful
- [ ] Known limitations documented

### 14.3 CHANGELOG.md
- [ ] v0.7.0 entry complete
- [ ] All macOS features listed
- [ ] Breaking changes noted (if any)
- [ ] Upgrade instructions clear

---

## 15. Known Issues Tracking

Document any issues found during testing:

### Critical Issues
- [ ] *List any critical bugs that block usage*

### Major Issues
- [ ] *List any major bugs that impact functionality*

### Minor Issues
- [ ] *List any minor bugs or inconveniences*

### Documentation Issues
- [ ] *List any documentation inaccuracies*

### Feature Requests
- [ ] *List any missing features discovered during testing*

---

## Test Results Summary

**Test Date:** _______________
**Tester:** _______________
**macOS Version:** _______________
**Hardware:** _______________
**Swictation Version:** _______________

### Overall Results
- **Total Tests:** _____ / _____
- **Passed:** _____
- **Failed:** _____
- **Blocked:** _____
- **Pass Rate:** _____%

### Critical Path Status
- [ ] Installation works end-to-end
- [ ] Accessibility permissions functional
- [ ] Text injection works in major apps
- [ ] GPU acceleration enabled
- [ ] Service management functional
- [ ] No Linux regressions

### Sign-off
- [ ] **Ready for release** (all critical tests pass)
- [ ] **Needs fixes** (critical issues found)
- [ ] **Blocked** (hardware/environment issues)

**Notes:**
_______________________________________________________________
_______________________________________________________________
_______________________________________________________________

---

## Appendix: Testing Tools

### Useful Commands
```bash
# Check system info
system_profiler SPSoftwareDataType SPHardwareDataType

# Monitor logs in real-time
tail -f ~/Library/Logs/swictation/daemon.log
tail -f ~/Library/Logs/swictation/daemon-error.log

# Check service status
launchctl list | grep swictation
ps aux | grep swictation

# Validate plist files
plutil -lint ~/Library/LaunchAgents/com.swictation.*.plist

# Monitor GPU usage
# Open Activity Monitor → GPU tab

# Check library dependencies
otool -L ~/.npm-global/lib/node_modules/swictation/lib/native/libonnxruntime.dylib

# Memory profiling
leaks swictation-daemon

# Measure inference time
time echo "test" | swictation --stdin
```

### Testing Scripts
Create automated test scripts in `tests/macos/`:
- `test_installation.sh` - Automated installation verification
- `test_text_injection.sh` - Simulate text injection scenarios
- `test_performance.sh` - Measure latency benchmarks
- `test_services.sh` - Verify service lifecycle

---

**Document Version:** 1.0
**Last Updated:** 2025-11-24
**Author:** Archon (AI Task Coordinator)
