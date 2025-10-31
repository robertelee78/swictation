#!/bin/bash
# Quick validation script for batch mode configuration

echo "=================================================="
echo "Swictation Batch Mode Validation"
echo "=================================================="

# Check if streaming mode is disabled
echo ""
echo "1️⃣  Checking default configuration..."

if grep -q "streaming_mode: bool = False" src/swictationd.py; then
    echo "   ✅ streaming_mode defaulted to False"
else
    echo "   ❌ streaming_mode still True (ERROR)"
    exit 1
fi

# Check AudioCapture initialization
echo ""
echo "2️⃣  Checking AudioCapture configuration..."

if grep -q "streaming_mode=False" src/swictationd.py; then
    echo "   ✅ AudioCapture initialized with streaming_mode=False"
else
    echo "   ⚠️  AudioCapture streaming_mode not explicitly set"
fi

# Quick Python validation
echo ""
echo "3️⃣  Running Python configuration check..."

python3 -c "
import sys
sys.path.insert(0, 'src')
from swictationd import SwictationDaemon

daemon = SwictationDaemon()
assert daemon.streaming_mode == False, 'Streaming mode should be disabled!'
print('   ✅ Python validation passed')
print(f'   ✅ streaming_mode = {daemon.streaming_mode}')
" || exit 1

# Check for test file
echo ""
echo "4️⃣  Checking test infrastructure..."

if [ -f "tests/test_batch_accuracy.py" ]; then
    echo "   ✅ Batch accuracy test exists"
else
    echo "   ❌ Batch accuracy test missing"
    exit 1
fi

# Check for documentation
echo ""
echo "5️⃣  Checking documentation..."

if [ -f "docs/BATCH_MODE_MIGRATION.md" ]; then
    echo "   ✅ Migration documentation exists"
else
    echo "   ⚠️  Migration documentation missing"
fi

echo ""
echo "=================================================="
echo "✅ Batch Mode Validation PASSED"
echo "=================================================="
echo ""
echo "Next steps:"
echo "  1. Record test audio: arecord -f S16_LE -r 16000 -c 1 -d 10 tests/data/fish_counting.wav"
echo "  2. Run accuracy test: python3 tests/test_batch_accuracy.py"
echo "  3. Test daemon: python3 src/swictationd.py"
echo ""
