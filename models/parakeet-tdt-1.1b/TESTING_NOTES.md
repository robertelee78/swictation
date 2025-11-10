# Parakeet-TDT 1.1B Testing Notes

## Export Status: ✅ SUCCESS

Exported successfully on 2025-11-10 using NVIDIA NeMo Docker container (25.07).

## Integration Status: ⚠️ PARTIAL - Needs Debugging

### What Works
- ✅ Model loads successfully with ONNX Runtime
- ✅ Encoder processes audio (80 mel features → 1024-dim output)
- ✅ Decoder and joiner execute without errors
- ✅ Inference completes in ~545ms on GPU

### What Doesn't Work
- ❌ Transcription produces nonsense output ("mmhmm" instead of real speech)
- ❌ Decoder outputs mostly blank tokens with few random tokens

## Technical Details

### Model Requirements
- **Mel Features**: 80 (NOT 128 like 0.6B model)
- **Encoder Input**: `[batch, 80, time]`
- **Encoder Output**: `[batch, 1024, time]`
- **Decoder**: 2 RNN layers, 640 hidden dimension
- **Vocabulary**: 1025 tokens including `<blk>` (blank)

### Files
```
encoder.int8.onnx    1.1GB  (INT8 quantized)
decoder.int8.onnx    7.0MB  (INT8 quantized)
joiner.int8.onnx     1.7MB  (INT8 quantized)
tokens.txt           11KB   (1025 tokens)
```

## Known Issues

### Issue #1: Incorrect Transcription

**Symptoms:**
- Input: "en-short.mp3" (normal speech)
- Output: "mmhmm" (nonsense)
- Token IDs: `[19, 1010, 1005, 1010, 1010]`

**Possible Causes:**
1. **Export Format Mismatch**: NeMo export may produce slightly different format than expected
2. **Decoder Logic Bug**: Our decoder/joiner implementation may not match 1.1B requirements
3. **Audio Preprocessing**: Subtle differences in mel-spectrogram computation
4. **Model-Specific Parameters**: 1.1B may have different expectations than 0.6B

**Debug Attempts:**
- ✅ Fixed mel feature count (128 → 80)
- ✅ Verified encoder input/output shapes
- ✅ Confirmed vocabulary format
- ❌ Cannot verify with sherpa-onnx CLI (installation issues)

## Comparison: 0.6B vs 1.1B

| Feature | 0.6B | 1.1B |
|---------|------|------|
| Mel Features | 128 | **80** |
| Parameters | 600M | 1.1B |
| Encoder Output Dim | 128 | **1024** |
| Works in Rust? | ✅ Yes | ⚠️ Partially |

## Next Steps

### Short Term (Recommended)
1. Use 0.6B model for production (proven to work)
2. Document 1.1B as "experimental"
3. Create test suite to verify correct transcription

### Long Term
1. Verify export with sherpa-onnx reference implementation
2. Compare our preprocessing with official NeMo preprocessing
3. Debug decoder/joiner logic with verbose logging
4. Consider using parakeet-rs if it supports 1.1B directly

## Testing Commands

### Build Test Binary
```bash
cargo build --release --example test_1_1b_direct
```

### Run Test
```bash
cargo run --release --example test_1_1b_direct /opt/swictation/examples/en-short.mp3
```

### Expected vs Actual
- **Expected**: Real transcription of audio content
- **Actual**: "mmhmm" (5 tokens, mostly blanks)

## References

- Export script: `/opt/swictation/scripts/export_parakeet_tdt_1.1b.py`
- Rust implementation: `/opt/swictation/rust-crates/swictation-stt/src/recognizer_ort.rs`
- Audio preprocessing: `/opt/swictation/rust-crates/swictation-stt/src/audio.rs`
- Test program: `/opt/swictation/rust-crates/swictation-stt/examples/test_1_1b_direct.rs`

## Conclusion

The 1.1B model export was successful, and we've learned critical details about its requirements. However, the Rust integration needs additional debugging to produce correct transcriptions. The 0.6B model remains the recommended choice for production use until 1.1B is fully validated.

---
*Last updated: 2025-11-10*
*Status: Blocked on transcription validation*
