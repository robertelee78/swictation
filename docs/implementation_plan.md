# Swictation Streaming Implementation Plan

**Date:** 2025-10-30
**Project:** fbeae03f-cd20-47a1-abf2-c9be91af34ca
**Parent Task:** c2205c00-f42a-4e7e-8e34-1dbd870b81e6

---

## Overview

This document outlines the complete implementation plan for production-quality streaming transcription using NeMo's Wait-k policy with Canary-1B-Flash.

**Goal:** Zero missed words, zero hallucinations, real-time text injection

**Status:** Ready to implement (detailed tasks created in Archon)

---

## Task Breakdown

### 1. Study NeMo Reference Script (Task: db67b328)
**Priority:** 89 | **Status:** TODO | **Assignee:** Claude

Study `speech_to_text_aed_streaming_infer.py` to understand:
- FrameBatchMultiTaskAED architecture
- AEDStreamingDecodingConfig parameters
- Wait-k policy decoder state management
- Hallucination detection mechanisms

**Deliverable:** `docs/nemo_streaming_architecture.md`

---

### 2. Create Standalone Test Script (Task: 8907d51e)
**Priority:** 88 | **Status:** TODO | **Assignee:** Claude

Build `tests/test_nemo_streaming.py` to verify NeMo streaming works correctly before daemon integration.

**Configuration:**
```python
streaming_cfg = DictConfig({
    'chunk_secs': 1.0,
    'left_context_secs': 10.0,
    'right_context_secs': 0.5,
    'decoding': {
        'streaming_policy': 'waitk',
        'waitk_lagging': 2,
        'hallucinations_detector': True
    }
})
```

**Success Criteria:**
- Transcribes "Hello world. Testing, one, two, three" with 100% accuracy
- Processes in 1-second chunks
- Maintains context across chunks

---

### 3. Refactor Audio Capture (Task: 6dd24324)
**Priority:** 87 | **Status:** TODO | **Assignee:** Claude

Modify `audio_capture.py` to yield 1-second chunks instead of 64ms blocks.

**Changes:**
- Add `chunk_duration=1.0` parameter
- Internal accumulator for 1-second chunks
- New `on_chunk_ready` callback (1Hz)
- Maintain backward compatibility with batch mode

**Testing:**
- Verify exact 1.0-second chunks (16000 samples)
- No gaps or overlaps between chunks
- Buffer doesn't overflow

---

### 4. Integrate FrameBatchMultiTaskAED (Task: ca748726)
**Priority:** 86 | **Status:** TODO | **Assignee:** Claude

Replace naive chunk processing with proper NeMo streaming in `swictationd.py`.

**Implementation:**
```python
from nemo.collections.asr.parts.utils.streaming_utils import FrameBatchMultiTaskAED

self.frame_asr = FrameBatchMultiTaskAED(
    asr_model=self.stt_model,
    frame_len=1.0,          # 1-second chunks
    total_buffer=10.0,      # 10-second left context
    batch_size=1,           # Real-time processing
)
```

**Key Features:**
- Stateful decoder (maintains context)
- Wait-k policy prevents hallucinations
- 10-second left context window

---

### 5. Progressive Text Injection (Task: 3b7d9eb8)
**Priority:** 85 | **Status:** TODO | **Assignee:** Claude

Implement delta-based text injection to avoid duplicates.

**Algorithm:**
```python
def inject_streaming_text(new_transcription: str):
    if new_transcription.startswith(self._last_injected):
        delta = new_transcription[len(self._last_injected):]
        if delta.strip():
            self.text_injector.inject(delta)
            self._last_injected = new_transcription
    else:
        # Transcription changed (correction)
        self.text_injector.inject(new_transcription)
        self._last_injected = new_transcription
```

**Edge Cases:**
- Corrections/revisions mid-stream
- Punctuation additions
- Capitalization changes
- Empty deltas

---

### 6. End-to-End Testing (Task: 2ae1e8ee)
**Priority:** 84 | **Status:** TODO | **Assignee:** Claude

Comprehensive test suite to validate accuracy.

**Test Cases:**
1. **Short audio** - en-short.mp3 (100% accuracy)
2. **Long recording** - 30+ seconds (WER < 1%)
3. **Real-time dictation** - Verify progressive injection
4. **Context preservation** - "The cat... The cat was orange"
5. **Hallucination detection** - Silent audio after speech

**Success Criteria:**
- ✅ 100% accuracy match with batch mode
- ✅ Zero hallucinations
- ✅ Zero omitted words
- ✅ Latency < 2 seconds
- ✅ Smooth text injection

---

### 7. Performance Optimization (Task: 9af88b6e)
**Priority:** 83 | **Status:** TODO | **Assignee:** Claude

Ensure efficient operation without memory leaks.

**Optimization Areas:**
1. GPU memory (stay < 4GB)
2. CPU/Thread efficiency
3. Latency (< 2s total)
4. Memory leak detection (10-minute test)
5. Error recovery

**Performance Targets:**
- GPU Memory: < 4GB
- CPU Usage: < 50%
- Latency: < 2s
- Stable over 10+ minute sessions

---

### 8. Documentation & Configuration (Task: 8d163fed)
**Priority:** 82 | **Status:** TODO | **Assignee:** Claude

Complete documentation and make streaming configurable.

**Documentation:**
- Update README.md
- Create `docs/streaming_implementation.md`
- Add inline code comments

**Configuration:**
```yaml
# config/streaming.yaml
streaming:
  enabled: true
  policy: waitk
  chunk_secs: 1.0
  left_context_secs: 10.0
  right_context_secs: 0.5
  waitk_lagging: 2
  hallucinations_detector: true
```

---

## Technical Architecture

### Current (Broken) Implementation
```
Audio (64ms) → Buffer (400ms) → Independent Transcription → Inject
                                        ↑
                                    NO CONTEXT
                                (Missing/Hallucinating words)
```

### Target (Wait-k) Implementation
```
Audio (1s chunk) → FrameBatchMultiTaskAED → Wait-k Decoder → Delta Inject
                            ↑                      ↑
                    10s Left Context      Stateful Decoder
                                         (Full Accuracy)
```

---

## Wait-k vs AlignAtt Comparison

| Feature | Wait-k (Our Choice) | AlignAtt |
|---------|-------------------|----------|
| Accuracy | ⭐⭐⭐⭐⭐ Higher | ⭐⭐⭐⭐ Good |
| Left Context | Infinite (best) | Fixed window |
| Latency | ~1.5-2s | ~1s |
| Hallucinations | Rare (detector enabled) | More common |
| Production Ready | ✅ Yes | ✅ Yes |
| Best For | **Dictation** (accuracy critical) | Real-time conversation |

**Decision:** Wait-k because accuracy > speed (user requirement)

---

## NeMo Configuration Parameters

### Core Streaming Settings
- `chunk_secs: 1.0` - Process audio in 1-second chunks
- `left_context_secs: 10.0` - Maintain 10 seconds of context (or infinite)
- `right_context_secs: 0.5` - Look-ahead buffer (affects latency)

### Wait-k Policy Settings
- `streaming_policy: waitk` - Use Wait-k algorithm
- `waitk_lagging: 2` - Wait for 2 chunks before starting transcription
- `hallucinations_detector: True` - Prevent false positive words

### Prompt Settings
- `+prompt.pnc: yes` - Enable punctuation and capitalization
- `+prompt.task: asr` - Automatic Speech Recognition task
- `+prompt.source_lang: en` - English input
- `+prompt.target_lang: en` - English output

---

## Reference Documentation

**Official NeMo Docs:**
https://docs.nvidia.com/nemo-framework/user-guide/latest/nemotoolkit/asr/streaming_decoding/canary_chunked_and_streaming_decoding.html

**Reference Script:**
`examples/asr/asr_chunked_inference/aed/speech_to_text_aed_streaming_infer.py`

**Research Document:**
`docs/streaming_research.md`

---

## Timeline Estimate

| Task | Estimated Time | Dependencies |
|------|---------------|--------------|
| Study Reference | 2-3 hours | None |
| Standalone Test | 2-3 hours | Study complete |
| Audio Refactor | 1-2 hours | Standalone test working |
| Integration | 3-4 hours | Audio refactor done |
| Text Injection | 1-2 hours | Integration working |
| Testing | 2-3 hours | Text injection done |
| Optimization | 2-3 hours | Testing complete |
| Documentation | 1-2 hours | Optimization done |

**Total:** 14-22 hours

**Note:** Not on time pressure - getting it right is priority

---

## Success Criteria

✅ **Accuracy:** 100% word-for-word match with batch mode
✅ **Reliability:** Zero hallucinations, zero omissions
✅ **Performance:** Latency < 2 seconds end-to-end
✅ **Stability:** Runs for 10+ minutes without issues
✅ **Privacy:** 100% local processing (no network calls)

---

## Fallback Plan

Current implementation (commit 4e4192e) serves as Option A:
- Basic streaming with 400ms chunks
- Works but has accuracy issues
- Can revert if needed

Batch mode (original) always available:
- Toggle on → record
- Toggle off → transcribe once
- 100% accuracy guaranteed

---

## Next Steps

1. Execute tasks in priority order (89 → 82)
2. Validate each step before moving forward
3. Document learnings and challenges
4. Iterate on configuration for optimal performance

**Current Status:** Ready to begin implementation
**Next Task:** Study NeMo reference script (Task db67b328)

---

*This is a living document - update as implementation progresses*
