#!/bin/bash
# Automated Testing Script for "mmhmm" Bug Fixes
# Tests Queen's hypothesis about per-feature normalization
# Part of: swarm-1762793181382-yexhcffpi

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$PROJECT_ROOT/rust-crates/swictation-stt"
AUDIO_RS="$RUST_DIR/src/audio.rs"
TEST_RESULTS="$PROJECT_ROOT/test-results"
TEST_AUDIO="$PROJECT_ROOT/examples/en-short.mp3"
EXPECTED_TEXT="hey there how are you doing today"

# Ensure test audio exists
if [ ! -f "$TEST_AUDIO" ]; then
    echo -e "${RED}ERROR: Test audio not found: $TEST_AUDIO${NC}"
    exit 1
fi

# Create test results directory
mkdir -p "$TEST_RESULTS"/{baseline,test-1,test-2,test-3,test-4,test-5/{combo-A,combo-B,combo-C,combo-D,combo-E}}

echo "========================================="
echo "🧪 Automated Test Suite for mmhmm Bug"
echo "========================================="
echo ""
echo "Project: $PROJECT_ROOT"
echo "Audio File: $TEST_AUDIO"
echo "Expected: $EXPECTED_TEXT"
echo ""

# Function to backup audio.rs
backup_audio_rs() {
    echo -e "${BLUE}📦 Backing up audio.rs...${NC}"
    cp "$AUDIO_RS" "$AUDIO_RS.backup"
}

# Function to restore audio.rs
restore_audio_rs() {
    echo -e "${BLUE}🔄 Restoring audio.rs...${NC}"
    if [ -f "$AUDIO_RS.backup" ]; then
        cp "$AUDIO_RS.backup" "$AUDIO_RS"
    fi
}

# Function to build Rust code
build_rust() {
    echo -e "${BLUE}🔨 Building Rust code...${NC}"
    cd "$RUST_DIR"
    cargo build --release 2>&1 | tee build.log | tail -5
    local build_status=${PIPESTATUS[0]}

    if [ $build_status -ne 0 ]; then
        echo -e "${RED}❌ Build failed!${NC}"
        cat build.log
        return 1
    fi

    echo -e "${GREEN}✅ Build successful${NC}"
    return 0
}

# Function to run transcription test
run_transcription() {
    local output_file="$1"
    local test_name="$2"

    echo -e "${BLUE}🎤 Running transcription test: $test_name${NC}"

    cd "$PROJECT_ROOT"

    # Run transcription and capture output
    timeout 60s cargo run --release --bin swictation-stt -- transcribe "$TEST_AUDIO" 2>&1 | tee "$output_file"
    local exit_code=${PIPESTATUS[0]}

    if [ $exit_code -eq 124 ]; then
        echo -e "${RED}⏱️  Test timed out (60 seconds)${NC}"
        echo "TIMEOUT" > "$output_file.status"
        return 1
    elif [ $exit_code -ne 0 ]; then
        echo -e "${RED}❌ Test crashed (exit code: $exit_code)${NC}"
        echo "CRASH:$exit_code" > "$output_file.status"
        return 1
    fi

    return 0
}

# Function to extract transcription from output
extract_transcription() {
    local output_file="$1"

    # Look for transcription in output
    # Typical format: "Transcription: text here" or just the text after processing
    grep -i "transcription" "$output_file" | tail -1 | sed 's/.*transcription[:]* //I' || \
    tail -1 "$output_file"
}

# Function to check if test passed
check_test_result() {
    local output_file="$1"
    local test_name="$2"

    echo -e "${BLUE}📊 Analyzing results for $test_name${NC}"

    # Extract transcription
    local actual_text=$(extract_transcription "$output_file")

    echo "Expected: $EXPECTED_TEXT"
    echo "Actual:   $actual_text"

    # Calculate similarity (simple word-based)
    local expected_words=$(echo "$EXPECTED_TEXT" | wc -w)
    local matching_words=0

    for word in $EXPECTED_TEXT; do
        if echo "$actual_text" | grep -iq "$word"; then
            ((matching_words++)) || true
        fi
    done

    local accuracy=$((matching_words * 100 / expected_words))

    echo "Accuracy: $accuracy% ($matching_words/$expected_words words)"

    # Check for "mmhmm" gibberish
    if echo "$actual_text" | grep -iq "mmhmm"; then
        echo -e "${RED}❌ FAIL: Still producing 'mmhmm' gibberish${NC}"
        echo "FAIL:mmhmm" > "$output_file.status"
        return 1
    fi

    # Check accuracy threshold
    if [ $accuracy -ge 70 ]; then
        echo -e "${GREEN}✅ PASS: Transcription is $accuracy% accurate${NC}"
        echo "PASS:$accuracy" > "$output_file.status"
        return 0
    else
        echo -e "${RED}❌ FAIL: Transcription is only $accuracy% accurate${NC}"
        echo "FAIL:$accuracy" > "$output_file.status"
        return 1
    fi
}

# Function to apply Test 1: Per-Feature Normalization
apply_test_1() {
    echo -e "${YELLOW}📝 Applying Test 1: Per-Feature Normalization${NC}"

    # Find the line "Ok(log_mel)" and insert normalization code before it
    # This is after the log_mel computation (around line 313-348)

    # Create the normalization code
    cat > /tmp/test1_code.txt << 'EOF'

        // TEST 1: Add per-feature normalization (matching reference implementation)
        // Normalize each mel feature dimension to mean=0, std=1 across time
        let num_frames = log_mel.nrows();
        let num_features = log_mel.ncols();

        debug!("Applying per-feature normalization (Test 1)");
        for feat_idx in 0..num_features {
            let mut column = log_mel.column_mut(feat_idx);
            let mean: f32 = column.iter().sum::<f32>() / num_frames as f32;
            let variance: f32 = column.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f32>() / num_frames as f32;
            let std = variance.sqrt().max(1e-10);

            for val in column.iter_mut() {
                *val = (*val - mean) / std;
            }
        }

        debug!("Per-feature normalization complete - mean={:.6}, std={:.6}",
               log_mel.mean().unwrap_or(0.0),
               {
                   let mean = log_mel.mean().unwrap_or(0.0);
                   let variance = log_mel.iter()
                       .map(|&x| (x - mean).powi(2))
                       .sum::<f32>() / (log_mel.len() as f32);
                   variance.sqrt()
               });
EOF

    # Insert before the final Ok(log_mel) in extract_mel_features
    # Find line with "Ok(log_mel)" after "no per-feature normalization" comment
    local insert_line=$(grep -n "Ok(log_mel)" "$AUDIO_RS" | tail -1 | cut -d: -f1)

    if [ -z "$insert_line" ]; then
        echo -e "${RED}ERROR: Could not find insertion point${NC}"
        return 1
    fi

    # Insert the code
    head -n $((insert_line - 1)) "$AUDIO_RS" > /tmp/audio_rs_new.rs
    cat /tmp/test1_code.txt >> /tmp/audio_rs_new.rs
    tail -n +$insert_line "$AUDIO_RS" >> /tmp/audio_rs_new.rs
    mv /tmp/audio_rs_new.rs "$AUDIO_RS"

    echo -e "${GREEN}✅ Test 1 code applied${NC}"
}

# Function to apply Test 2: Window Function
apply_test_2() {
    echo -e "${YELLOW}📝 Applying Test 2: Hann Window${NC}"

    # Replace povey_window with hann_window in compute_stft
    sed -i 's/let window = povey_window(WIN_LENGTH);/let window = hann_window(WIN_LENGTH); \/\/ TEST 2: Hann window/' "$AUDIO_RS"

    echo -e "${GREEN}✅ Test 2 code applied${NC}"
}

# Function to apply Test 3: Frequency Range
apply_test_3() {
    echo -e "${YELLOW}📝 Applying Test 3: Frequency Range 0-8000 Hz${NC}"

    # Replace frequency range parameters
    sed -i 's/20\.0,    \/\/ sherpa-onnx uses low_freq=20/0.0,      \/\/ TEST 3: Reference uses 0 Hz/' "$AUDIO_RS"
    sed -i 's/7600\.0,  \/\/ sherpa-onnx uses high_freq=8000-400=7600/8000.0,   \/\/ TEST 3: Reference uses SR\/2/' "$AUDIO_RS"

    echo -e "${GREEN}✅ Test 3 code applied${NC}"
}

# Function to apply Test 4: Remove Sample Normalization
apply_test_4() {
    echo -e "${YELLOW}📝 Applying Test 4: Skip Sample Normalization${NC}"

    # Comment out normalize_audio_samples call
    sed -i 's/let normalized_samples = normalize_audio_samples(samples);/let normalized_samples = samples.to_vec(); \/\/ TEST 4: Skip normalization/' "$AUDIO_RS"

    echo -e "${GREEN}✅ Test 4 code applied${NC}"
}

# Function to run a single test
run_test() {
    local test_num="$1"
    local test_name="$2"
    local apply_func="$3"
    local results_dir="$TEST_RESULTS/test-$test_num"

    echo ""
    echo "========================================="
    echo -e "${YELLOW}🧪 Test $test_num: $test_name${NC}"
    echo "========================================="

    # Restore clean audio.rs
    restore_audio_rs

    # Apply test modification
    $apply_func

    # Create diff
    diff -u "$AUDIO_RS.backup" "$AUDIO_RS" > "$results_dir/audio.rs.diff" || true

    # Build
    if ! build_rust > "$results_dir/build.log" 2>&1; then
        echo -e "${RED}❌ Build failed for Test $test_num${NC}"
        echo "FAIL:build" > "$results_dir/status"
        return 1
    fi

    # Run transcription
    if ! run_transcription "$results_dir/output.txt" "$test_name"; then
        echo -e "${RED}❌ Transcription failed for Test $test_num${NC}"
        return 1
    fi

    # Check results
    if check_test_result "$results_dir/output.txt" "$test_name"; then
        echo -e "${GREEN}✅✅✅ Test $test_num PASSED! Bug is FIXED! ✅✅✅${NC}"
        echo "PASS" > "$results_dir/status"

        # Copy successful diff to root
        cp "$results_dir/audio.rs.diff" "$TEST_RESULTS/SUCCESSFUL_FIX.diff"

        return 0
    else
        echo -e "${RED}❌ Test $test_num FAILED${NC}"
        echo "FAIL" > "$results_dir/status"
        return 1
    fi
}

# Function to run combination test
run_combo_test() {
    local combo_name="$1"
    shift
    local test_funcs=("$@")
    local results_dir="$TEST_RESULTS/test-5/$combo_name"

    echo ""
    echo "========================================="
    echo -e "${YELLOW}🧪 Combination Test: $combo_name${NC}"
    echo "========================================="

    # Restore clean audio.rs
    restore_audio_rs

    # Apply all test modifications
    for func in "${test_funcs[@]}"; do
        $func
    done

    # Create diff
    diff -u "$AUDIO_RS.backup" "$AUDIO_RS" > "$results_dir/audio.rs.diff" || true

    # Build
    if ! build_rust > "$results_dir/build.log" 2>&1; then
        echo -e "${RED}❌ Build failed for $combo_name${NC}"
        echo "FAIL:build" > "$results_dir/status"
        return 1
    fi

    # Run transcription
    if ! run_transcription "$results_dir/output.txt" "$combo_name"; then
        echo -e "${RED}❌ Transcription failed for $combo_name${NC}"
        return 1
    fi

    # Check results
    if check_test_result "$results_dir/output.txt" "$combo_name"; then
        echo -e "${GREEN}✅✅✅ $combo_name PASSED! Bug is FIXED! ✅✅✅${NC}"
        echo "PASS" > "$results_dir/status"

        # Copy successful diff to root
        cp "$results_dir/audio.rs.diff" "$TEST_RESULTS/SUCCESSFUL_FIX.diff"

        return 0
    else
        echo -e "${RED}❌ $combo_name FAILED${NC}"
        echo "FAIL" > "$results_dir/status"
        return 1
    fi
}

# Main test execution
main() {
    echo -e "${BLUE}🚀 Starting automated test suite${NC}"

    # Backup original file
    backup_audio_rs

    # Run baseline test
    echo ""
    echo "========================================="
    echo -e "${BLUE}📊 Baseline Test (Current Broken State)${NC}"
    echo "========================================="

    if ! build_rust > "$TEST_RESULTS/baseline/build.log" 2>&1; then
        echo -e "${RED}ERROR: Baseline build failed${NC}"
        restore_audio_rs
        exit 1
    fi

    run_transcription "$TEST_RESULTS/baseline/output.txt" "Baseline"
    check_test_result "$TEST_RESULTS/baseline/output.txt" "Baseline" || true

    echo -e "${BLUE}Baseline established. Starting fix tests...${NC}"

    # Test 1: Per-Feature Normalization (PRIORITY 1)
    if run_test "1" "Per-Feature Normalization" "apply_test_1"; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH TEST 1! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Missing per-feature normalization"
        echo "Solution: Add per-feature normalization across time dimension"
        echo ""
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Test 2: Window Function (PRIORITY 2)
    if run_test "2" "Window Function (Hann)" "apply_test_2"; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH TEST 2! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Incorrect window function"
        echo "Solution: Use Hann window instead of Povey window"
        echo ""
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Test 3: Frequency Range (PRIORITY 3)
    if run_test "3" "Frequency Range (0-8000 Hz)" "apply_test_3"; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH TEST 3! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Incorrect frequency range"
        echo "Solution: Use 0-8000 Hz instead of 20-7600 Hz"
        echo ""
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Test 4: Sample Normalization (PRIORITY 4)
    if run_test "4" "Remove Sample Normalization" "apply_test_4"; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH TEST 4! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Excessive sample normalization"
        echo "Solution: Remove sample normalization step"
        echo ""
        restore_audio_rs
        generate_report
        exit 0
    fi

    # No individual test passed, try combinations
    echo ""
    echo -e "${YELLOW}⚠️  No individual test passed. Trying combinations...${NC}"

    # Combination A: Test 1 only (already tried above, but include for completeness)

    # Combination B: Test 1 + 2
    if run_combo_test "combo-B" apply_test_1 apply_test_2; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH COMBO B (Test 1 + 2)! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Per-feature normalization + window function"
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Combination C: Test 1 + 3
    if run_combo_test "combo-C" apply_test_1 apply_test_3; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH COMBO C (Test 1 + 3)! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Per-feature normalization + frequency range"
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Combination D: Test 1 + 2 + 3
    if run_combo_test "combo-D" apply_test_1 apply_test_2 apply_test_3; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH COMBO D (Test 1 + 2 + 3)! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Multiple mel-spectrogram processing issues"
        restore_audio_rs
        generate_report
        exit 0
    fi

    # Combination E: All tests (1 + 2 + 3 + 4)
    if run_combo_test "combo-E" apply_test_1 apply_test_2 apply_test_3 apply_test_4; then
        echo ""
        echo -e "${GREEN}🎉🎉🎉 BUG FIXED WITH COMBO E (All tests)! 🎉🎉🎉${NC}"
        echo ""
        echo "Root Cause: Multiple audio processing issues"
        restore_audio_rs
        generate_report
        exit 0
    fi

    # If we get here, all tests failed
    echo ""
    echo -e "${RED}❌❌❌ ALL TESTS FAILED ❌❌❌${NC}"
    echo ""
    echo "None of the hypothesized fixes resolved the issue."
    echo "Deeper investigation required."
    echo ""

    restore_audio_rs
    generate_report
    exit 1
}

# Function to generate final report
generate_report() {
    echo ""
    echo "========================================="
    echo "📊 Test Summary Report"
    echo "========================================="
    echo ""

    cat > "$TEST_RESULTS/summary-report.md" << EOF
# Test Summary Report
**Date:** $(date)
**Test Audio:** $TEST_AUDIO
**Expected Output:** "$EXPECTED_TEXT"

## Test Results

### Baseline (Current Broken State)
- Status: $(cat "$TEST_RESULTS/baseline/status" 2>/dev/null || echo "UNKNOWN")
- Output: $(extract_transcription "$TEST_RESULTS/baseline/output.txt" 2>/dev/null || echo "UNKNOWN")

### Test 1: Per-Feature Normalization
- Status: $(cat "$TEST_RESULTS/test-1/status" 2>/dev/null || echo "NOT_RUN")
- Output: $(extract_transcription "$TEST_RESULTS/test-1/output.txt" 2>/dev/null || echo "N/A")

### Test 2: Window Function (Hann)
- Status: $(cat "$TEST_RESULTS/test-2/status" 2>/dev/null || echo "NOT_RUN")
- Output: $(extract_transcription "$TEST_RESULTS/test-2/output.txt" 2>/dev/null || echo "N/A")

### Test 3: Frequency Range (0-8000 Hz)
- Status: $(cat "$TEST_RESULTS/test-3/status" 2>/dev/null || echo "NOT_RUN")
- Output: $(extract_transcription "$TEST_RESULTS/test-3/output.txt" 2>/dev/null || echo "N/A")

### Test 4: Remove Sample Normalization
- Status: $(cat "$TEST_RESULTS/test-4/status" 2>/dev/null || echo "NOT_RUN")
- Output: $(extract_transcription "$TEST_RESULTS/test-4/output.txt" 2>/dev/null || echo "N/A")

### Combination Tests
- Combo B (1+2): $(cat "$TEST_RESULTS/test-5/combo-B/status" 2>/dev/null || echo "NOT_RUN")
- Combo C (1+3): $(cat "$TEST_RESULTS/test-5/combo-C/status" 2>/dev/null || echo "NOT_RUN")
- Combo D (1+2+3): $(cat "$TEST_RESULTS/test-5/combo-D/status" 2>/dev/null || echo "NOT_RUN")
- Combo E (1+2+3+4): $(cat "$TEST_RESULTS/test-5/combo-E/status" 2>/dev/null || echo "NOT_RUN")

## Successful Fix

EOF

    if [ -f "$TEST_RESULTS/SUCCESSFUL_FIX.diff" ]; then
        echo "A successful fix was found! See: $TEST_RESULTS/SUCCESSFUL_FIX.diff"
        echo ""
        echo "\`\`\`diff" >> "$TEST_RESULTS/summary-report.md"
        cat "$TEST_RESULTS/SUCCESSFUL_FIX.diff" >> "$TEST_RESULTS/summary-report.md"
        echo "\`\`\`" >> "$TEST_RESULTS/summary-report.md"
    else
        echo "No successful fix was found in this test run." >> "$TEST_RESULTS/summary-report.md"
    fi

    echo ""
    echo "Full report saved to: $TEST_RESULTS/summary-report.md"
}

# Run main function
main "$@"
