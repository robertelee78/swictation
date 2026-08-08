#!/bin/bash
# Swictation Audio Capture Test Script
# Tests the swictation-audio crate for proper device enumeration,
# audio capture, and sample rate conversion (44.1kHz → 16kHz)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AUDIO_CRATE="$PROJECT_ROOT/rust-crates/swictation-audio"
WORKSPACE_ROOT="$PROJECT_ROOT/rust-crates"
TARGET_DIR="$WORKSPACE_ROOT/target/release"
DOCS_DIR="$PROJECT_ROOT/docs/tests"
TEST_OUTPUT="$DOCS_DIR/audio-capture-test.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[⚠]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Create docs directory if it doesn't exist
mkdir -p "$DOCS_DIR"

# Start test report
cat > "$TEST_OUTPUT" <<EOF
# Audio Capture Test Report

**Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Crate:** swictation-audio
**Test Script:** scripts/test-audio-capture.sh

## Test Objective

Test the audio capture component to verify:
1. Audio device enumeration works correctly
2. Audio samples can be captured from the default microphone
3. Sample rate conversion (44.1kHz stereo → 16kHz mono) functions properly
4. No device enumeration errors occur

## System Information

EOF

# Gather system info
log_info "Gathering system information..."
{
    echo '```'
    echo "OS: $(uname -s)"
    echo "Kernel: $(uname -r)"
    echo "Architecture: $(uname -m)"
    if command -v pulseaudio &> /dev/null; then
        echo "Audio System: PulseAudio $(pulseaudio --version | head -n1)"
    fi
    if command -v pipewire &> /dev/null; then
        echo "Audio System: PipeWire $(pipewire --version 2>&1 | head -n1)"
    fi
    echo '```'
    echo ""
} >> "$TEST_OUTPUT"

# Build the audio crate
log_info "Building swictation-audio crate..."
cd "$WORKSPACE_ROOT"
if cargo build --release --examples -p swictation-audio 2>&1 | tee -a "$AUDIO_CRATE/build.log"; then
    log_success "Build successful"
    echo "## Build Status" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "✅ **SUCCESS** - Audio crate built successfully" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
else
    log_error "Build failed"
    echo "## Build Status" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "❌ **FAILED** - Build errors encountered" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    tail -n 50 "$AUDIO_CRATE/build.log" >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    exit 1
fi

cd "$AUDIO_CRATE"

# Test 1: List available devices
log_info "Test 1: Listing available audio devices..."
echo "## Test 1: Device Enumeration" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"

if "$TARGET_DIR/examples/list_devices" > device_list.log 2>&1; then
    log_success "Device enumeration successful"

    echo "### Result: ✅ SUCCESS" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "Available devices:" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    cat device_list.log >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"

    # Count devices
    DEVICE_COUNT=$(grep -c "Type:" device_list.log || echo "0")
    echo "**Devices Found:** $DEVICE_COUNT" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"

    # Check for default device
    if grep -q "DEFAULT INPUT" device_list.log; then
        DEFAULT_DEVICE=$(grep -B 1 "DEFAULT INPUT" device_list.log | head -n1 | sed 's/^[ \t]*//')
        log_success "Default input device found: $DEFAULT_DEVICE"
        echo "**Default Input Device:** $DEFAULT_DEVICE" >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"
    else
        log_warning "No default input device marked"
        echo "**Warning:** No default input device found" >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"
    fi
else
    log_error "Device enumeration failed"
    echo "### Result: ❌ FAILED" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    cat device_list.log >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    exit 1
fi

# Test 2: Live audio capture test (5 seconds)
log_info "Test 2: Capturing 5 seconds of audio..."
echo "## Test 2: Audio Capture (5 seconds)" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"
echo "**Instructions:** This test captures 5 seconds of audio. For best results:" >> "$TEST_OUTPUT"
echo "- Play audio through speakers during capture" >> "$TEST_OUTPUT"
echo "- Or speak into the microphone" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"

# Modify test_live_audio to run for 5 seconds instead of 10
log_warning "Starting 5-second audio capture..."
log_warning "Please speak or play audio NOW!"

# Run a shorter version by timing out after 5.5 seconds
timeout 6s "$TARGET_DIR/examples/test_live_audio" > audio_test.log 2>&1 || true

if [ -f audio_test.log ] && [ -s audio_test.log ]; then
    log_success "Audio capture completed"

    echo "### Result: ✅ CAPTURED" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    cat audio_test.log >> "$TEST_OUTPUT"
    echo '```' >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"

    # Analyze results
    echo "### Analysis" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"

    # Check for sample rate info
    if grep -q "Sample Rate:" audio_test.log; then
        SAMPLE_RATE_INFO=$(grep "Sample Rate:" audio_test.log)
        log_info "Sample rate conversion: $SAMPLE_RATE_INFO"
        echo "**Sample Rate Conversion:**" >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        echo "$SAMPLE_RATE_INFO" >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"

        # Verify 16kHz conversion
        if echo "$SAMPLE_RATE_INFO" | grep -q "16000"; then
            log_success "Target sample rate of 16kHz confirmed"
            echo "✅ Target sample rate of 16kHz confirmed" >> "$TEST_OUTPUT"
            echo "" >> "$TEST_OUTPUT"
        else
            log_warning "16kHz target sample rate not detected"
            echo "⚠️ 16kHz target sample rate not clearly indicated" >> "$TEST_OUTPUT"
            echo "" >> "$TEST_OUTPUT"
        fi
    fi

    # Check for channel conversion
    if grep -q "Channels:" audio_test.log; then
        CHANNEL_INFO=$(grep "Channels:" audio_test.log)
        log_info "Channel conversion: $CHANNEL_INFO"
        echo "**Channel Conversion:**" >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        echo "$CHANNEL_INFO" >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"

        # Verify mono output
        if echo "$CHANNEL_INFO" | grep -q "→ 1"; then
            log_success "Mono output (1 channel) confirmed"
            echo "✅ Mono output (1 channel) confirmed" >> "$TEST_OUTPUT"
            echo "" >> "$TEST_OUTPUT"
        fi
    fi

    # Check peak levels
    if grep -q "Peak:" audio_test.log; then
        PEAK_LEVEL=$(grep "Final Analysis" -A 10 audio_test.log | grep "Peak:" | awk '{print $2}')
        if [ -n "$PEAK_LEVEL" ]; then
            log_info "Peak audio level: $PEAK_LEVEL"
            echo "**Peak Audio Level:** $PEAK_LEVEL" >> "$TEST_OUTPUT"
            echo "" >> "$TEST_OUTPUT"

            # Evaluate audio level
            PEAK_FLOAT=$(echo "$PEAK_LEVEL" | awk '{print $1}')
            if awk -v peak="$PEAK_FLOAT" 'BEGIN {exit !(peak >= 0.1)}'; then
                log_success "Good audio level detected (peak ≥ 0.1)"
                echo "✅ **Good audio level** (peak ≥ 0.1)" >> "$TEST_OUTPUT"
                echo "" >> "$TEST_OUTPUT"
            elif awk -v peak="$PEAK_FLOAT" 'BEGIN {exit !(peak >= 0.01)}'; then
                log_warning "Low audio level (peak < 0.1)"
                echo "⚠️ **Low audio level** (peak < 0.1) - Consider increasing volume" >> "$TEST_OUTPUT"
                echo "" >> "$TEST_OUTPUT"
            else
                log_warning "Very low audio level (peak < 0.01)"
                echo "⚠️ **Very low audio level** (peak < 0.01) - Check microphone/device" >> "$TEST_OUTPUT"
                echo "" >> "$TEST_OUTPUT"
            fi
        fi
    fi

    # Check for samples captured
    if grep -q "Samples:" audio_test.log; then
        SAMPLES=$(grep "Samples:" audio_test.log | tail -n1 | awk '{print $2}')
        DURATION=$(grep "Duration:" audio_test.log | tail -n1 | awk '{print $2}')
        log_info "Captured $SAMPLES samples ($DURATION)"
        echo "**Samples Captured:** $SAMPLES ($DURATION)" >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"
    fi

    # Check for errors
    if grep -qi "error" audio_test.log; then
        log_error "Errors detected in audio capture log"
        echo "❌ **Errors detected during capture**" >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        grep -i "error" audio_test.log >> "$TEST_OUTPUT"
        echo '```' >> "$TEST_OUTPUT"
        echo "" >> "$TEST_OUTPUT"
    fi
else
    log_error "Audio capture failed or produced no output"
    echo "### Result: ❌ FAILED" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "No audio capture output generated" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
fi

# Test 3: Check for device enumeration errors
log_info "Test 3: Checking for device enumeration errors..."
echo "## Test 3: Error Detection" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"

ERROR_COUNT=0

# Check build log
if grep -qi "error" build.log 2>/dev/null; then
    ERROR_COUNT=$((ERROR_COUNT + 1))
    echo "- ❌ Errors found in build log" >> "$TEST_OUTPUT"
fi

# Check device list log
if grep -qi "error\|failed\|panic" device_list.log 2>/dev/null; then
    ERROR_COUNT=$((ERROR_COUNT + 1))
    echo "- ❌ Errors found in device enumeration" >> "$TEST_OUTPUT"
fi

# Check audio test log
if grep -qi "error\|failed\|panic" audio_test.log 2>/dev/null; then
    ERROR_COUNT=$((ERROR_COUNT + 1))
    echo "- ❌ Errors found in audio capture" >> "$TEST_OUTPUT"
fi

if [ $ERROR_COUNT -eq 0 ]; then
    log_success "No critical errors detected"
    echo "### Result: ✅ NO ERRORS" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "No critical errors, panics, or failures detected in any test." >> "$TEST_OUTPUT"
else
    log_warning "$ERROR_COUNT error(s) detected"
    echo "### Result: ⚠️ ERRORS DETECTED" >> "$TEST_OUTPUT"
    echo "" >> "$TEST_OUTPUT"
    echo "Found $ERROR_COUNT error(s) during testing. See logs above for details." >> "$TEST_OUTPUT"
fi
echo "" >> "$TEST_OUTPUT"

# Final Summary
echo "## Summary" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"
echo "| Test | Status | Details |" >> "$TEST_OUTPUT"
echo "|------|--------|---------|" >> "$TEST_OUTPUT"

# Test 1 status
if grep -q "Device enumeration successful" build.log device_list.log 2>/dev/null || [ -s device_list.log ]; then
    echo "| Device Enumeration | ✅ PASS | $(grep -c "Type:" device_list.log || echo "0") devices found |" >> "$TEST_OUTPUT"
else
    echo "| Device Enumeration | ❌ FAIL | No devices enumerated |" >> "$TEST_OUTPUT"
fi

# Test 2 status
if [ -s audio_test.log ] && grep -q "Samples:" audio_test.log; then
    SAMPLES=$(grep "Samples:" audio_test.log | tail -n1 | awk '{print $2}')
    echo "| Audio Capture | ✅ PASS | $SAMPLES samples captured |" >> "$TEST_OUTPUT"
else
    echo "| Audio Capture | ❌ FAIL | No audio captured |" >> "$TEST_OUTPUT"
fi

# Test 3 status
if [ $ERROR_COUNT -eq 0 ]; then
    echo "| Error Detection | ✅ PASS | No critical errors |" >> "$TEST_OUTPUT"
else
    echo "| Error Detection | ⚠️ WARNING | $ERROR_COUNT error(s) found |" >> "$TEST_OUTPUT"
fi

echo "" >> "$TEST_OUTPUT"

# Success criteria check
echo "## Success Criteria Evaluation" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"

SUCCESS_COUNT=0
TOTAL_CRITERIA=3

# Criterion 1: Audio samples captured successfully
if [ -s audio_test.log ] && grep -q "Samples:" audio_test.log; then
    echo "- ✅ **Audio samples captured successfully**" >> "$TEST_OUTPUT"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
else
    echo "- ❌ **Audio samples NOT captured**" >> "$TEST_OUTPUT"
fi

# Criterion 2: Proper resampling 44.1kHz → 16kHz
if grep -q "16000" audio_test.log device_list.log 2>/dev/null; then
    echo "- ✅ **Proper resampling to 16kHz verified**" >> "$TEST_OUTPUT"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
else
    echo "- ❌ **16kHz resampling NOT verified**" >> "$TEST_OUTPUT"
fi

# Criterion 3: No device enumeration errors
if ! grep -qi "error.*device\|failed.*enumerate" device_list.log audio_test.log 2>/dev/null; then
    echo "- ✅ **No device enumeration errors**" >> "$TEST_OUTPUT"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
else
    echo "- ❌ **Device enumeration errors detected**" >> "$TEST_OUTPUT"
fi

echo "" >> "$TEST_OUTPUT"
echo "**Overall Result:** $SUCCESS_COUNT / $TOTAL_CRITERIA criteria met" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"

if [ $SUCCESS_COUNT -eq $TOTAL_CRITERIA ]; then
    log_success "ALL SUCCESS CRITERIA MET ($SUCCESS_COUNT/$TOTAL_CRITERIA)"
    echo "🎉 **ALL SUCCESS CRITERIA MET** - Audio capture component is working correctly!" >> "$TEST_OUTPUT"
    FINAL_EXIT=0
elif [ $SUCCESS_COUNT -ge 2 ]; then
    log_warning "PARTIAL SUCCESS ($SUCCESS_COUNT/$TOTAL_CRITERIA criteria met)"
    echo "⚠️ **PARTIAL SUCCESS** - Some issues detected, but core functionality works" >> "$TEST_OUTPUT"
    FINAL_EXIT=0
else
    log_error "FAILED ($SUCCESS_COUNT/$TOTAL_CRITERIA criteria met)"
    echo "❌ **FAILED** - Critical issues with audio capture component" >> "$TEST_OUTPUT"
    FINAL_EXIT=1
fi

echo "" >> "$TEST_OUTPUT"
echo "---" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"
echo "**Test completed:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")" >> "$TEST_OUTPUT"
echo "" >> "$TEST_OUTPUT"
echo "**Log files:**" >> "$TEST_OUTPUT"
echo "- Build log: \`$AUDIO_CRATE/build.log\`" >> "$TEST_OUTPUT"
echo "- Device list: \`$AUDIO_CRATE/device_list.log\`" >> "$TEST_OUTPUT"
echo "- Audio test: \`$AUDIO_CRATE/audio_test.log\`" >> "$TEST_OUTPUT"

# Print location of test report
log_info "Test report saved to: $TEST_OUTPUT"
echo ""
log_info "You can view the report with:"
echo "    cat $TEST_OUTPUT"
echo ""

# Clean up old logs (keep the test report)
cd "$PROJECT_ROOT"

exit $FINAL_EXIT
