#!/bin/bash
set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Error handler
error_exit() {
    echo -e "${RED}❌ ERROR: $1${NC}" >&2
    exit 1
}

# Progress indicator
progress() {
    echo -e "${CYAN}▶ $1${NC}"
}

# Success indicator
success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Warning indicator
warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Header
header() {
    echo -e "${BLUE}================================================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}================================================================================${NC}"
}

# Main script
header "🔬 MEL FEATURE EXTRACTION COMPARISON: Rust vs Python"
echo ""

# Configuration
AUDIO_FILE="${1:-/opt/swictation/examples/en-short.mp3}"
RUST_CSV="/tmp/rust_mel_features.csv"
PYTHON_CSV="/tmp/python_mel_features.csv"
COMPARISON_REPORT="/tmp/mel_comparison_report.txt"

echo "📂 Configuration:"
echo "   Audio file:        $AUDIO_FILE"
echo "   Rust output:       $RUST_CSV"
echo "   Python output:     $PYTHON_CSV"
echo "   Comparison report: $COMPARISON_REPORT"
echo ""

# Validate inputs
if [ ! -f "$AUDIO_FILE" ]; then
    error_exit "Audio file not found: $AUDIO_FILE"
fi

# Check if Rust binary exists
if [ ! -f "/opt/swictation/rust-crates/target/release/examples/export_mel_features" ]; then
    warning "Rust binary not found, attempting to build..."
    progress "Building Rust project..."
    cd /opt/swictation/rust-crates
    cargo build --release --example export_mel_features || error_exit "Failed to build Rust project"
    cd - > /dev/null
    success "Rust project built successfully"
    echo ""
fi

# Check if Python scripts exist
if [ ! -f "/opt/swictation/scripts/extract_python_mel_features.py" ]; then
    error_exit "Python extraction script not found: /opt/swictation/scripts/extract_python_mel_features.py"
fi

if [ ! -f "/opt/swictation/scripts/compare_mel_features.py" ]; then
    error_exit "Comparison script not found: /opt/swictation/scripts/compare_mel_features.py"
fi

# Clean up previous outputs
progress "Cleaning up previous outputs..."
rm -f "$RUST_CSV" "$PYTHON_CSV" "$COMPARISON_REPORT"

# Step 1: Extract features with Rust
header "🦀 STEP 1: Extracting Mel Features with Rust"
echo ""

progress "Running Rust feature extractor..."
/opt/swictation/rust-crates/target/release/examples/export_mel_features \
    "$AUDIO_FILE" "$RUST_CSV" || error_exit "Rust feature extraction failed"

if [ ! -f "$RUST_CSV" ]; then
    error_exit "Rust CSV output not created: $RUST_CSV"
fi

# Validate Rust CSV
RUST_LINE_COUNT=$(wc -l < "$RUST_CSV")
if [ "$RUST_LINE_COUNT" -lt 2 ]; then
    error_exit "Rust CSV appears empty or invalid (only $RUST_LINE_COUNT lines)"
fi

success "Rust features exported successfully"
echo "   Lines in CSV: $RUST_LINE_COUNT"
echo "   First few lines:"
head -n 3 "$RUST_CSV" | sed 's/^/   /'
echo ""

# Step 2: Extract features with Python
header "🐍 STEP 2: Extracting Mel Features with Python sherpa-onnx"
echo ""

progress "Running Python feature extractor..."
python3.12 /opt/swictation/scripts/extract_python_mel_features.py \
    "$AUDIO_FILE" "$PYTHON_CSV" || error_exit "Python feature extraction failed"

if [ ! -f "$PYTHON_CSV" ]; then
    error_exit "Python CSV output not created: $PYTHON_CSV"
fi

# Validate Python CSV
PYTHON_LINE_COUNT=$(wc -l < "$PYTHON_CSV")
if [ "$PYTHON_LINE_COUNT" -lt 2 ]; then
    error_exit "Python CSV appears empty or invalid (only $PYTHON_LINE_COUNT lines)"
fi

success "Python features exported successfully"
echo "   Lines in CSV: $PYTHON_LINE_COUNT"
echo "   First few lines:"
head -n 3 "$PYTHON_CSV" | sed 's/^/   /'
echo ""

# Step 3: Compare features
header "📊 STEP 3: Comparing Features Element-by-Element"
echo ""

progress "Running detailed comparison analysis..."
python3.12 /opt/swictation/scripts/compare_mel_features.py \
    "$RUST_CSV" "$PYTHON_CSV" | tee "$COMPARISON_REPORT"

# Check comparison exit code
COMPARISON_EXIT=$?

echo ""
header "🎯 DIAGNOSIS COMPLETE"
echo ""

# Summary
if [ $COMPARISON_EXIT -eq 0 ]; then
    success "Feature comparison completed successfully"
else
    warning "Feature comparison detected differences"
fi

echo ""
echo "📋 Output Files Generated:"
echo "   • Rust features:      $RUST_CSV ($RUST_LINE_COUNT lines)"
echo "   • Python features:    $PYTHON_CSV ($PYTHON_LINE_COUNT lines)"
echo "   • Comparison report:  $COMPARISON_REPORT"
echo ""

echo "🔍 Next Steps for Debugging:"
echo ""
if grep -q "MATCH" "$COMPARISON_REPORT" 2>/dev/null; then
    echo "   ✓ Features MATCH: Bug is likely in encoder/decoder stage"
    echo "     → Check ONNX Runtime tensor format"
    echo "     → Verify encoder input/output shapes"
    echo "     → Compare CTC decoder beam search parameters"
    echo "     → Inspect token probability distributions"
else
    echo "   ✗ Features DIFFER: Bug is in mel extraction stage"
    echo "     → Check STFT window/hop length parameters"
    echo "     → Verify mel filterbank configuration"
    echo "     → Compare normalization methods"
    echo "     → Inspect audio preprocessing (resampling, padding)"
fi
echo ""

echo "📚 Manual Inspection Commands:"
echo "   • View Rust CSV:    less $RUST_CSV"
echo "   • View Python CSV:  less $PYTHON_CSV"
echo "   • Compare side-by-side: diff -y $RUST_CSV $PYTHON_CSV | less"
echo ""

header "🏁 All diagnostics complete!"
echo ""
