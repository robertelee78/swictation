# Code Quality Analysis Report: Model Selection Inconsistencies

**Analysis Date**: 2025-11-21
**Context**: Post-sherpa-rs removal (Task #101)
**Scope**: VRAM threshold logic, model path references, and naming consistency

---

## Executive Summary

**Overall Quality Score**: 6/10
**Critical Issues Found**: 5
**Technical Debt Estimate**: 4-6 hours

After the removal of sherpa-rs in Task #101, the codebase contains significant inconsistencies in VRAM threshold logic and model naming conventions. While both 0.6B and 1.1B models now correctly use OrtRecognizer, there are **conflicting VRAM thresholds** across different files that could lead to incorrect model selection decisions.

---

## Critical Issues

### 1. VRAM Threshold Inconsistencies (HIGH SEVERITY)

**Location**: Multiple files
**Impact**: Model selection may fail or select wrong model variant

#### Conflicting Thresholds:

| Component | File | Line | 1.1B Threshold | 0.6B Threshold | Status |
|-----------|------|------|----------------|----------------|--------|
| **postinstall.js** (✓ SOURCE OF TRUTH) | `npm-package/postinstall.js` | 1136, 1140 | **6000 MB** | **3500 MB** | CORRECT |
| pipeline.rs (✓ CORRECT) | `rust-crates/swictation-daemon/src/pipeline.rs` | 147, 166 | **6000 MB** | **3500 MB** | CORRECT |
| engine.rs (❌ WRONG) | `rust-crates/swictation-stt/src/engine.rs` | 163-164, 179, 186 | **4096 MB** | **1536 MB** | INCONSISTENT |
| gpu.rs tests (❌ WRONG) | `rust-crates/swictation-daemon/src/gpu.rs` | 234, 249 | **4096 MB** | **1536 MB** | INCONSISTENT |
| architecture.md (❌ WRONG) | `docs/architecture.md` | 378, 385 | **4096 MB** | **1536 MB** | INCONSISTENT |

**Evidence**:

```rust
// ❌ WRONG - engine.rs (lines 163-164, 179, 186)
/// - `4096` (4GB) for 1.1B INT8 GPU model (peak 3.5GB + 500MB headroom)
/// - `1536` (1.5GB) for 0.6B GPU model (peak 1.2GB + 300MB headroom)
    4096 // 4GB minimum for 1.1B INT8 GPU
    1536 // 1.5GB minimum for 0.6B GPU

// ❌ WRONG - gpu.rs (lines 234, 249)
let threshold_1_1b = 4096;  // 4GB minimum threshold
let threshold_0_6b = 1536;  // 1.5GB minimum threshold

// ✓ CORRECT - postinstall.js (lines 1136, 1140)
if (gpuInfo.vramMB >= 6000) {
  gpuInfo.recommendedModel = '1.1b-gpu';
} else if (gpuInfo.vramMB >= 3500) {
  gpuInfo.recommendedModel = '0.6b-gpu';

// ✓ CORRECT - pipeline.rs (lines 147, 166)
if vram >= 6000 {
    info!("✓ Sufficient VRAM for 1.1B INT8 model (requires ≥6GB)");
} else if vram >= 3500 {
    info!("✓ Sufficient VRAM for 0.6B GPU model (requires ≥3.5GB)");
```

**Root Cause**: Engine.rs and gpu.rs were not updated when postinstall.js thresholds were changed based on real-world RTX A1000 (4GB) and RTX PRO 6000 Blackwell (97GB) testing.

**Impact**:
- RTX 3060 (12GB VRAM): Would incorrectly select 1.1B at 4GB threshold instead of correct 6GB
- RTX A1000 (4GB VRAM): ✓ Would correctly select 0.6B at both thresholds
- RTX 2060 (6GB VRAM): Ambiguous - might select wrong model

**Recommendation**: Update engine.rs and gpu.rs to use 6000/3500 thresholds to match postinstall.js.

---

### 2. Model Path Naming Inconsistencies (MEDIUM SEVERITY)

**Location**: Multiple files
**Impact**: Configuration mismatch, documentation confusion

#### Inconsistent Directory Names:

| File | Variable | Value | Status |
|------|----------|-------|--------|
| postinstall.js | config paths | `parakeet-tdt-1.1b-onnx` | ✓ CORRECT |
| postinstall.js | model map | `sherpa-onnx-nemo-parakeet-tdt-1.1b-v3-onnx` | ❌ WRONG |
| architecture.md | example path | `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx` | Mixed |

**Evidence**:

```javascript
// ❌ INCONSISTENT - postinstall.js lines 1214-1215 vs 1718-1722
// Config template (CORRECT):
stt_1_1b_model_path = "${modelDir}/parakeet-tdt-1.1b-onnx"

// Model directory map (WRONG - contains sherpa-onnx prefix):
const modelDirs = {
  '1.1b': 'sherpa-onnx-nemo-parakeet-tdt-1.1b-v3-onnx',  // ❌ Wrong
  '1.1b-gpu': 'sherpa-onnx-nemo-parakeet-tdt-1.1b-v3-onnx',  // ❌ Wrong
};
```

**Root Cause**: Model directory map still uses old sherpa-onnx naming convention, but actual config paths use new parakeet-tdt naming.

**Recommendation**: Standardize on `parakeet-tdt-*` naming everywhere.

---

### 3. Dead Code Path in Documentation (LOW SEVERITY)

**Location**: `docs/architecture.md`
**Lines**: 378-399
**Impact**: Misleading documentation showing old sherpa-rs path

**Evidence**:

```rust
// ❌ WRONG - architecture.md shows old Recognizer::new (sherpa-rs API)
} else if vram >= 1536 {
    // Moderate VRAM: Use 0.6B GPU for good quality (7-8% WER)
    info!("✓ Sufficient VRAM for 0.6B GPU model (requires ≥1.5GB)");
    let recognizer = Recognizer::new(&config.stt_0_6b_model_path, true)?;  // ❌ OLD API
    info!("✓ Parakeet-TDT-0.6B loaded successfully (GPU)");
    SttEngine::Parakeet0_6B(recognizer)
```

**Actual Code** (pipeline.rs):

```rust
// ✓ CORRECT - All model loading now uses OrtRecognizer
let ort_recognizer = OrtRecognizer::new(&config.stt_0_6b_model_path, true)
```

**Recommendation**: Update architecture.md code examples to use `OrtRecognizer::new` API.

---

### 4. Model Selection Logic Tests Have Wrong Thresholds (MEDIUM SEVERITY)

**Location**: `rust-crates/swictation-daemon/src/gpu.rs`
**Lines**: 233-349 (test functions)
**Impact**: Tests validate wrong thresholds, giving false confidence

**Evidence**:

```rust
// ❌ WRONG - Tests use 4096/1536 instead of 6000/3500
#[test]
fn test_vram_thresholds() {
    let threshold_1_1b = 4096;  // ❌ Should be 6000
    let threshold_0_6b = 1536;  // ❌ Should be 3500

    // Tests validate wrong expectations:
    let selected = if tc.vram_mb >= 6000 {  // ✓ Runtime uses 6000
        "1.1B GPU INT8"
    } else if tc.vram_mb >= 3500 {  // ✓ Runtime uses 3500
        "0.6B GPU"
    } else {
        "0.6B CPU"
    };

    assert_eq!(selected, tc.expected_model, ...);
    // ❌ But test cases expect 4096/1536 thresholds!
}
```

**Root Cause**: Test constants not updated when runtime logic changed to 6000/3500.

**Recommendation**: Update test constants to 6000/3500 to match runtime behavior.

---

### 5. Old sherpa-onnx References in Model Names (LOW SEVERITY)

**Location**: Multiple documentation and download scripts
**Impact**: Confusion about model sources

**Affected Files**:
- `npm-package/postinstall.js` - lines 1718-1722 (model map)
- `docs/architecture.md` - lines 431, 434, 502, 514
- `config/config.example.toml` - lines 43, 46
- Various download scripts still reference `sherpa-onnx-nemo-parakeet-tdt` naming

**Evidence**:

```toml
# config.example.toml - Mix of sherpa-onnx and parakeet-tdt naming
stt_0_6b_model_path = "~/.local/share/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-onnx"
stt_1_1b_model_path = "~/.local/share/swictation/models/parakeet-tdt-1.1b-onnx"
```

**Recommendation**: Standardize all model path references to use consistent naming convention.

---

## Code Smells Detected

### 1. Magic Numbers (VRAM Thresholds)
- **Type**: Duplicated Constants
- **Location**: engine.rs, gpu.rs, pipeline.rs, postinstall.js
- **Smell**: Same threshold values hardcoded in 4+ locations
- **Suggestion**: Create shared constants module or config file

### 2. Long Method (Model Selection Logic)
- **Location**: `pipeline.rs` lines 140-220 (80+ lines)
- **Complexity**: Nested if-else with error handling
- **Suggestion**: Extract model selection to dedicated function

### 3. Duplicate Code (VRAM Threshold Checks)
- **Pattern**: Same `if vram >= X` pattern repeated in 3 files
- **Suggestion**: Centralize threshold logic in single source file

### 4. Inconsistent Naming
- **Pattern**: Mix of `sherpa-onnx-nemo-parakeet-tdt` and `parakeet-tdt`
- **Suggestion**: Choose one naming convention and apply everywhere

---

## Refactoring Opportunities

### 1. Centralize VRAM Threshold Configuration (High Impact)

**Benefit**: Single source of truth, eliminates inconsistencies

```rust
// NEW: rust-crates/swictation-core/src/constants.rs
pub mod vram_thresholds {
    /// Minimum VRAM for 1.1B INT8 model (6GB)
    /// Peak usage: 3.5GB, Threshold: 6GB = 2.5GB headroom (42%)
    /// Source: Real-world testing on RTX PRO 6000 Blackwell
    pub const MIN_1_1B_GPU_MB: u64 = 6000;

    /// Minimum VRAM for 0.6B GPU model (3.5GB)
    /// Peak usage: 1.2GB, Threshold: 3.5GB = 2.3GB headroom (66%)
    /// Source: Real-world testing on RTX A1000 4GB
    pub const MIN_0_6B_GPU_MB: u64 = 3500;

    /// Model peak memory usage (for testing/validation)
    pub const PEAK_1_1B_MB: u64 = 3500;
    pub const PEAK_0_6B_MB: u64 = 1200;
}
```

### 2. Extract Model Selection Logic (Medium Impact)

**Benefit**: Testable, reusable, clearer separation of concerns

```rust
// NEW: rust-crates/swictation-stt/src/model_selector.rs
pub struct ModelSelector {
    vram_mb: Option<u64>,
}

impl ModelSelector {
    pub fn select_model(&self, config: &Config) -> Result<SttEngine> {
        match self.vram_mb {
            Some(vram) if vram >= vram_thresholds::MIN_1_1B_GPU_MB => {
                self.load_1_1b_gpu(config)
            }
            Some(vram) if vram >= vram_thresholds::MIN_0_6B_GPU_MB => {
                self.load_0_6b_gpu(config)
            }
            _ => {
                self.load_0_6b_cpu(config)
            }
        }
    }
}
```

### 3. Standardize Model Path Constants (Low Impact)

**Benefit**: Consistent naming, easier maintenance

```rust
// NEW: rust-crates/swictation-core/src/model_paths.rs
pub const MODEL_0_6B_DIR: &str = "parakeet-tdt-0.6b-v3-onnx";
pub const MODEL_1_1B_DIR: &str = "parakeet-tdt-1.1b-onnx";
pub const MODEL_0_6B_INT8_DIR: &str = "parakeet-tdt-0.6b-v3-onnx-int8";
```

---

## Positive Findings

### ✓ Correct Implementations

1. **pipeline.rs** (runtime model selection):
   - ✓ Uses correct 6000/3500 thresholds
   - ✓ All models use OrtRecognizer (no sherpa-rs dependency)
   - ✓ Clear error messages with troubleshooting guidance

2. **postinstall.js** (installation):
   - ✓ Correct thresholds documented from real-world testing
   - ✓ Clear comments explaining headroom calculations
   - ✓ Proper GPU detection and model recommendation

3. **engine.rs** (API design):
   - ✓ Clean enum-based SttEngine abstraction
   - ✓ Unified interface for both model sizes
   - ✓ Good documentation with usage examples

### ✓ Good Practices Observed

- Comprehensive error messages with troubleshooting steps
- Real-world testing on production hardware (RTX A1000, RTX PRO 6000)
- Clear documentation of WER scores and latency expectations
- Proper headroom calculations (42% for 1.1B, 66% for 0.6B)

---

## Verification Checklist

To verify fixes, check these key points:

- [ ] All VRAM thresholds use 6000/3500 (not 4096/1536)
- [ ] All code uses OrtRecognizer::new (not Recognizer::new)
- [ ] Model paths consistently use `parakeet-tdt-*` naming
- [ ] Tests validate correct 6000/3500 thresholds
- [ ] Documentation reflects actual OrtRecognizer API
- [ ] No references to sherpa-rs or SherpaRecognizer in active code
- [ ] postinstall.js model map matches actual directory names

---

## Files Requiring Updates

### High Priority (Incorrect Thresholds):

1. `/opt/swictation/rust-crates/swictation-stt/src/engine.rs`
   - Lines 163-164: Update doc comments (4096→6000, 1536→3500)
   - Lines 179, 186: Update threshold constants

2. `/opt/swictation/rust-crates/swictation-daemon/src/gpu.rs`
   - Lines 234, 249: Update test threshold constants
   - Lines 286-327: Update test case expectations

3. `/opt/swictation/docs/architecture.md`
   - Lines 378, 385: Update VRAM threshold examples
   - Lines 388, 396: Update Recognizer::new to OrtRecognizer::new
   - Line 534: Update minimum VRAM documentation

### Medium Priority (Naming Inconsistencies):

4. `/opt/swictation/npm-package/postinstall.js`
   - Lines 1718-1722: Fix model directory map naming

5. `/opt/swictation/config/config.example.toml`
   - Lines 43, 46: Standardize model path naming

### Low Priority (Documentation/Examples):

6. Various example files still showing old sherpa-onnx naming
7. Download scripts using mixed naming conventions

---

## Recommended Action Plan

### Phase 1: Critical Fixes (1-2 hours)

1. Create shared constants module for VRAM thresholds
2. Update engine.rs to use 6000/3500 thresholds
3. Update gpu.rs tests to validate correct thresholds
4. Update pipeline.rs comments if needed

### Phase 2: Consistency Improvements (2-3 hours)

1. Standardize model naming across all files
2. Update architecture.md with correct code examples
3. Fix postinstall.js model directory map
4. Update config.example.toml

### Phase 3: Code Quality (1-2 hours)

1. Extract model selection logic to dedicated module
2. Create shared model path constants
3. Add integration tests for threshold logic
4. Update all documentation

---

## Summary

The codebase has **inconsistent VRAM thresholds** that pose a **real risk** of incorrect model selection on GPUs with 4-8GB VRAM. While the runtime code (pipeline.rs) uses correct thresholds, the documentation and test code use outdated values.

**Immediate Action Required**:
- Update engine.rs and gpu.rs to use 6000/3500 thresholds
- Fix test cases to validate correct behavior
- Update documentation to reflect actual implementation

**Long-term Improvements**:
- Centralize threshold constants
- Standardize model naming conventions
- Extract model selection logic for better testability
