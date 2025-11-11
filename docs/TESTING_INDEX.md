# 📚 Testing Documentation Index
## Quick Navigation for Rust STT Verification

**Created:** 2025-11-10
**Agent:** Tester (Hive Mind)
**Total Documentation:** 2,160 lines across 4 files

---

## 🎯 Start Here

### For Implementation Team
**Read this first:** [IMMEDIATE_TEST_PLAN.md](./IMMEDIATE_TEST_PLAN.md)
- Step-by-step execution guide
- Complete code samples
- 3-4 hour timeline
- Ready to run immediately

### For Project Managers
**Read this first:** [TESTER_AGENT_SUMMARY.md](./TESTER_AGENT_SUMMARY.md)
- Mission overview
- Deliverables summary
- Timeline and risk assessment
- Success metrics

### For Developers During Testing
**Use this:** [TEST_CHECKLIST.md](./TEST_CHECKLIST.md)
- Checkbox format
- Track progress
- Quick reference
- File creation list

### For Deep Dive / Reference
**Comprehensive guide:** [VERIFICATION_TESTING_STRATEGY.md](./VERIFICATION_TESTING_STRATEGY.md)
- 23 detailed test specifications
- Success criteria
- Expected outputs
- All 5 test categories

---

## 📁 Document Details

### 1. VERIFICATION_TESTING_STRATEGY.md
**Size:** 794 lines (22 KB)
**Purpose:** Comprehensive testing strategy
**Audience:** QA, senior developers, architects

**Contents:**
- Executive summary
- 5 test categories (Feature, Encoder, Decoder, Integration, Performance)
- 23 detailed test specifications
- Code samples and expected outputs
- Success criteria and metrics
- Verification checklist for fixes
- Testing philosophy

**When to read:**
- Need to understand full test coverage
- Designing new tests
- Reviewing test results
- Planning regression suite

---

### 2. IMMEDIATE_TEST_PLAN.md
**Size:** 730 lines (27 KB)
**Purpose:** Step-by-step implementation guide
**Audience:** Implementation team, developers

**Contents:**
- 4-step diagnostic sequence
- Complete Rust code samples
- Complete Python comparison scripts
- Master diagnostic script
- Expected results and failure modes
- Timeline (30 min per step)

**When to read:**
- Ready to start testing NOW
- Need code samples to implement
- Want sequential execution plan
- Following diagnostic workflow

**Key Sections:**
- Step 1: Raw audio loading (30 min)
- Step 2: Power spectrum (1 hour)
- Step 3: Mel filterbank (1 hour)
- Step 4: Unified diagnostic (30 min)

---

### 3. TEST_CHECKLIST.md
**Size:** 204 lines (7 KB)
**Purpose:** Progress tracking during implementation
**Audience:** Developers actively testing

**Contents:**
- Checkbox lists for all tests
- File creation checklist (11 files)
- Success metrics
- Red flags and warnings
- Coordination protocol

**When to use:**
- During active testing
- Tracking completion status
- Ensuring nothing is missed
- Quick status check

**Format:**
```
- [ ] Test 1.1: Raw audio loading
  - [ ] Create example file
  - [ ] Run test
  - [ ] Verify result
```

---

### 4. TESTER_AGENT_SUMMARY.md
**Size:** 432 lines (12 KB)
**Purpose:** Mission summary and handoff
**Audience:** Project managers, team leads

**Contents:**
- Mission objective and deliverables
- Test categories overview
- Critical success metrics
- Root cause hypothesis
- Execution workflow
- Timeline and risk assessment
- Final recommendations

**When to read:**
- Planning work assignment
- Estimating timeline
- Understanding scope
- Post-mortem analysis

---

## 🚀 Quick Start Guide

### Immediate Actions (Next 30 minutes)
1. Read [IMMEDIATE_TEST_PLAN.md](./IMMEDIATE_TEST_PLAN.md) - Steps 1-4
2. Open [TEST_CHECKLIST.md](./TEST_CHECKLIST.md) - Track progress
3. Start with Test 1.1 (raw audio)

### First Test Execution
```bash
# Follow IMMEDIATE_TEST_PLAN.md Step 1

# 1. Create scripts/compare_raw_audio.py
# 2. Add export_raw_samples() to audio.rs
# 3. Create examples/export_raw_audio.rs
# 4. Run:
cd /opt/swictation/rust-crates/swictation-stt
cargo run --example export_raw_audio --release examples/en-short.mp3 rust_raw_audio.csv

cd /opt/swictation
python scripts/compare_raw_audio.py rust_raw_audio.csv examples/en-short.mp3

# 5. Check TEST_CHECKLIST.md and mark complete
```

### After First Test
- If PASS: Continue to Test 1.2 (power spectrum)
- If FAIL: Debug audio loading/resampling
- Always coordinate via hooks:
  ```bash
  npx claude-flow@alpha hooks notify --message "Test 1.1: [result]"
  ```

---

## 📊 Test Overview

### Critical Path (Must Complete First)
```
Test 1.1 → Test 1.2 → Test 1.3 → Test 1.4
 (30min)    (1 hour)   (1 hour)   (exists)

              ↓
         Root Cause Identified
              ↓
         Implement Fix
              ↓
         Test 4.1 (transcription)
              ↓
         SUCCESS ✅
```

### Total Test Count: 23 tests
- **Category 1:** 4 tests (feature extraction)
- **Category 2:** 3 tests (encoder)
- **Category 3:** 2 tests (decoder)
- **Category 4:** 3 tests (integration)
- **Category 5:** 2 tests (performance)
- **Additional:** 9 tests (edge cases, comparison, etc.)

---

## 🎯 Success Criteria

### Phase 1: Isolate Root Cause
- ✅ One of Tests 1.1-1.3 identifies exact issue
- ✅ Confidence level: HIGH (>85%)

### Phase 2: Validate Fix
- ✅ Test 1.4: Mel features match Python (<0.1 dB)
- ✅ Test 4.1: Correct transcription
- ✅ Correlation >0.99

### Phase 3: Production Ready
- ✅ All 23 tests pass
- ✅ WER within 2% of Python
- ✅ No memory leaks
- ✅ Performance maintained

---

## 📂 Related Documentation

### Diagnosis Documents (Background)
- [HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md](./HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md)
- [aha-23-mel-offset-investigation.md](./aha-23-mel-offset-investigation.md)
- [decoder-blank-token-analysis.md](./decoder-blank-token-analysis.md)
- [rust-ort-decoder-bug-analysis.md](./rust-ort-decoder-bug-analysis.md)
- [rust-python-audio-analysis.md](./rust-python-audio-analysis.md)

### Implementation Files
- `rust-crates/swictation-stt/src/audio.rs` - Audio processing
- `rust-crates/swictation-stt/src/recognizer_ort.rs` - ONNX recognizer
- `scripts/diagnose_feature_mismatch.sh` - Existing diagnostic

---

## 🔗 File Dependencies

### Scripts to Create (Priority Order)
1. `scripts/compare_raw_audio.py` ⚡ START HERE
2. `examples/export_raw_audio.rs` ⚡ START HERE
3. `scripts/compare_power_spectrum.py`
4. `examples/export_power_spectrum.rs`
5. `scripts/verify_mel_filterbank.py`
6. `examples/export_mel_filterbank.rs`
7. `scripts/diagnose_root_cause.sh` (master script)

### Rust Code Modifications Needed
**File:** `rust-crates/swictation-stt/src/audio.rs`

Add these methods:
1. `export_raw_samples()` - Test 1.1
2. `export_power_spectrum_frame()` - Test 1.2
3. `create_povey_window()` - Test 1.2 helper
4. `export_mel_filterbank()` - Test 1.3

All code samples provided in IMMEDIATE_TEST_PLAN.md

---

## 💡 Tips for Success

### Do's ✅
- Follow steps sequentially
- Use provided code samples
- Coordinate via hooks after each test
- Document unexpected results
- Compare with Python baseline

### Don'ts ❌
- Skip tests (each narrows search space)
- Ignore small differences (<0.1 dB matters)
- Assume tests pass without verification
- Modify test code without documenting
- Proceed to next phase if tests fail

---

## 📞 Coordination

### Before Each Test
```bash
npx claude-flow@alpha hooks pre-task --description "Running Test X.Y: [name]"
```

### After Each Test
```bash
npx claude-flow@alpha hooks post-task --memory-key "hive/tester/test-X-Y"
npx claude-flow@alpha hooks notify --message "Test X.Y: [PASS/FAIL] - [finding]"
```

### Memory Keys
- `hive/tester/strategy-complete` - Strategy document
- `hive/tester/test-1-1` through `test-5-2` - Individual test results
- `hive/tester/diagnostic-results` - Overall findings

---

## 📈 Timeline Estimates

### Phase 1: Isolate (2-4 hours)
- Test 1.1: 30 minutes
- Test 1.2: 1 hour
- Test 1.3: 1 hour
- Test 1.4: Already exists
- **Total:** 2.5-4 hours

### Phase 2: Fix + Validate (4-8 hours)
- Implement fix: 2-4 hours
- Re-run tests: 1-2 hours
- Integration test: 1-2 hours
- **Total:** 4-8 hours

### Phase 3: Comprehensive (2-4 hours)
- All remaining tests: 2-3 hours
- Documentation: 1 hour
- **Total:** 2-4 hours

**Grand Total:** 8-16 hours

---

## 🎓 Testing Philosophy

From VERIFICATION_TESTING_STRATEGY.md:

> Tests are a safety net that enables confident refactoring and prevents regressions.
> Invest in good tests—they pay dividends in maintainability.

**Principles:**
1. **Isolate then integrate** - Fix components before full pipeline
2. **Compare with ground truth** - Python sherpa-onnx is the baseline
3. **Automate everything** - Scripts should run with one command
4. **Document deviations** - Explain all differences from Python
5. **No false positives** - 100% pass or documented failure

---

## 🏆 Hive Mind Context

**Swarm ID:** `swarm-1762761203503-wfwa5u13a`
**Task ID:** `task-1762791854801-xf0ano457`
**Memory:** `.swarm/memory.db`

**Coordination:**
- Pre-task hook: ✅ Executed
- Post-task hook: ✅ Executed
- Notify hook: ✅ Executed
- Session-end hook: ✅ Executed

**Dependencies:**
- Built on findings from Researcher, Analyst, Coder agents
- Provides validation framework for all agents

---

## ✅ Ready to Start

**Status:** All documentation complete
**Blocker:** None
**Next Action:** Test 1.1 (raw audio loading)
**Time Estimate:** 30 minutes for first test

**Good luck! The hive is with you.**

---

## 📞 Need Help?

### If Tests Behave Unexpectedly
1. Check VERIFICATION_TESTING_STRATEGY.md for detailed diagnostics
2. Review HIVE-MIND-DIAGNOSIS-RUST-MMHMM-BUG.md for context
3. Coordinate with hive via hooks
4. Document findings and ask for help

### If Timeline Slips
1. Don't skip tests to save time
2. Focus on critical path (Tests 1.1-1.3)
3. Implement fix before comprehensive validation
4. Prioritize Test 4.1 (end-to-end proof)

### If Root Cause Still Unclear After Phase 1
1. Review all test results together
2. Look for patterns across failures
3. Consult aha-23-mel-offset-investigation.md
4. Consider multiple interacting issues (rare)

---

**Documentation Index Complete**
**Ready for systematic debugging**
**Tester Agent standing by for results**
