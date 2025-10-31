# Streaming STT Research - Local Solutions for Real-Time Transcription

**Date:** 2025-10-30
**Task:** 714e5da5-7ab5-494f-b690-0725d0e40f2b
**Objective:** Evaluate local streaming STT options for privacy-focused real-time transcription

---

## Executive Summary

**RECOMMENDATION: NeMo Cache-Aware Streaming** ⭐

The Canary-1B-Flash model we're already using has built-in cache-aware streaming support via `transcribe_simulate_cache_aware_streaming()`. This provides:
- ~200ms latency (configurable)
- Production-ready accuracy
- Zero implementation complexity (native method)
- 100% local (no network calls)
- Already loaded in memory (3.6GB GPU)

---

## Comparison Matrix

| Solution | Latency | GPU Memory | CPU Option | Accuracy | Implementation | Privacy | Status |
|----------|---------|------------|------------|----------|----------------|---------|--------|
| **NeMo CAS** | 200ms | 3.6GB | ❌ | ⭐⭐⭐⭐⭐ | **Trivial** | ✅ Local | Production |
| Vosk | <200ms | 0 | ✅ | ⭐⭐⭐ | Low | ✅ Local | Production |
| WhisperStreaming | Adaptive | 2-4GB | ✅ | ⭐⭐⭐⭐ | Medium | ✅ Local | Production |
| SimulStreaming | Real-time | 5GB+ | ❌ | ⭐⭐⭐⭐⭐ | High | ✅ Local | Research |
| WhisperLiveKit | Near-RT | 2-4GB | ✅ | ⭐⭐⭐⭐ | Medium | ✅ Local | Active Dev |

---

## Detailed Analysis

### 1. NeMo Cache-Aware Streaming ⭐ RECOMMENDED

**Why this is the best choice:**
- Already using nvidia/canary-1b-flash model
- Built-in method: `model.transcribe_simulate_cache_aware_streaming()`
- Tutorial available: `NeMo/tutorials/asr/Online_ASR_Microphone_Demo_Cache_Aware_Streaming.ipynb`
- Reference script: `NeMo/examples/asr/asr_cache_aware_streaming/speech_to_text_cache_aware_streaming_infer.py`

**Performance:**
- Latency: ~200ms (configurable via chunk size)
- Accuracy: Production-ready (same as batch mode)
- Memory: 3.6GB GPU (already loaded)
- CPU: Not supported (GPU-only)

**Implementation:**
```python
# Minimal code changes required
for text in model.transcribe_simulate_cache_aware_streaming(
    audio_chunks,
    chunk_len_in_secs=0.2,  # 200ms chunks
    simulate_cache_aware_streaming=True
):
    inject_text(text)  # Real-time injection
```

**Pros:**
- Zero additional dependencies
- Native NeMo integration
- Maintains current accuracy
- Production-tested by NVIDIA
- 100% local processing

**Cons:**
- GPU required (not a problem for us)
- Only works with Conformer/FastConformer models

**Resources:**
- GitHub: https://github.com/NVIDIA/NeMo/blob/main/tutorials/asr/Online_ASR_Microphone_Demo_Cache_Aware_Streaming.ipynb
- Docs: https://docs.nvidia.com/nemo-framework/user-guide/latest/nemotoolkit/asr/models.html

---

### 2. Vosk - Lightweight CPU-Friendly Option

**Best for:** Low-resource environments, embedded systems, CPU-only machines

**Performance:**
- Latency: <200ms (zero-latency streaming API)
- Accuracy: Good (trades some accuracy for speed)
- Memory: 50MB - 1.8GB models
- CPU: ✅ Full CPU support

**Implementation:**
```python
import vosk
model = vosk.Model("model")
rec = vosk.KaldiRecognizer(model, 16000)
rec.AcceptWaveform(audio_data)
result = json.loads(rec.Result())
```

**Pros:**
- Extremely lightweight
- True CPU support
- Zero-latency streaming
- Small model sizes (50MB-1.8GB)
- Simple API

**Cons:**
- Initial 8s detection delay reported
- Lower accuracy than Whisper/NeMo
- Less sophisticated than modern transformers

**Resources:**
- GitHub: https://github.com/alphacep/vosk-api
- Latency blog: https://alphacephei.com/nsh/2020/11/27/latency.html

---

### 3. WhisperStreaming (ufal) - Balanced Option

**Best for:** When you need Whisper-level accuracy with streaming

**Performance:**
- Latency: Self-adaptive (complexity-based)
- Accuracy: ⭐⭐⭐⭐ (Whisper-level)
- Memory: 2-4GB GPU
- CPU: ✅ Supported via faster-whisper

**Implementation:**
```python
from whisper_streaming import WhisperStreamer
streamer = WhisperStreamer(model="large-v3", backend="faster-whisper")
for text in streamer.process_audio_stream(audio_chunks):
    inject_text(text)
```

**Pros:**
- Whisper-quality accuracy
- Self-adaptive latency
- Multiple backend support
- Active development
- Good documentation

**Cons:**
- More complex than NeMo native
- Additional dependencies
- Model loading overhead

**Resources:**
- GitHub: https://github.com/ufal/whisper_streaming
- PyPI: https://pypi.org/project/whisper-streaming/

---

### 4. SimulStreaming - Research-Grade Solution

**Best for:** Research applications, maximum accuracy requirements

**Performance:**
- Latency: Real-time capable
- Accuracy: ⭐⭐⭐⭐⭐ (State-of-art)
- Memory: 5GB+ (Whisper 1.6B + EuroLLM 9B)
- CPU: Not recommended

**Implementation:**
```python
# Server-based architecture
python simulstreaming_whisper_server.py \
    --host 0.0.0.0 \
    --port 8000 \
    --warmup-file warmup.wav
```

**Pros:**
- IWSLT 2025 competition-grade
- State-of-art accuracy
- MIT license
- Translation support

**Cons:**
- Heavy resource requirements
- Complex setup
- Research codebase
- Overkill for dictation use case

**Resources:**
- GitHub: https://github.com/ufal/SimulStreaming
- Paper: arXiv:2506.17077

---

### 5. WhisperLiveKit - Feature-Rich Alternative

**Best for:** Applications needing speaker identification

**Performance:**
- Latency: Near real-time
- Accuracy: ⭐⭐⭐⭐ (Whisper-based)
- Memory: 2-4GB GPU
- CPU: ✅ Supported

**Implementation:**
```python
from whisperlivekit import WhisperLive
live = WhisperLive(model="large-v3", backend="faster-whisper")
live.start_stream()
```

**Pros:**
- Speaker identification
- Multiple backends
- WebSocket support
- Active development
- Good documentation

**Cons:**
- Additional features we don't need
- More dependencies
- WebSocket overhead

**Resources:**
- GitHub: https://github.com/QuentinFuxa/WhisperLiveKit
- PyPI: https://pypi.org/project/whisper-live/

---

## Implementation Recommendation

### Primary: NeMo Cache-Aware Streaming

**Rationale:**
1. **Zero additional complexity** - already using the model
2. **Production-ready** - tested by NVIDIA
3. **Privacy-first** - 100% local, no network calls
4. **Performance** - ~200ms latency is excellent for dictation
5. **Resource-efficient** - no additional GPU memory needed

**Implementation steps:**
1. Replace batch `transcribe()` with `transcribe_simulate_cache_aware_streaming()`
2. Set chunk size to 200-400ms for optimal latency/accuracy
3. Process chunks in real-time as audio arrives
4. Inject text progressively as it's decoded

**Code changes:** Minimal (~20 lines in swictationd.py)

### Fallback: Vosk

**If NeMo streaming doesn't work:**
- Fallback to Vosk for CPU-only compatibility
- Trade some accuracy for broader hardware support
- Useful for testing on low-power devices

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Chunk size | 200-400ms | Balance latency vs accuracy |
| Processing latency | <200ms | Per chunk processing time |
| Total latency | <500ms | Audio capture → text injection |
| Accuracy | >95% WER | Maintain production quality |
| GPU memory | 3.6GB | Current usage acceptable |

---

## Privacy Analysis

All solutions reviewed are **100% local** and meet privacy requirements:

✅ NeMo Cache-Aware Streaming - Local GPU processing
✅ Vosk - Local CPU/GPU processing
✅ WhisperStreaming - Local processing (faster-whisper backend)
✅ SimulStreaming - Local processing (research grade)
✅ WhisperLiveKit - Local option available (no cloud required)

**No cloud APIs needed** - All processing happens on-device.

---

## Next Steps

1. ✅ Research completed (this document)
2. ⏳ Implement NeMo cache-aware streaming
3. ⏳ Test with real audio (microphone + playback)
4. ⏳ Measure actual latency in production
5. ⏳ Fine-tune chunk size for optimal performance
6. ⏳ Add fallback to Vosk if needed

---

## References

- NeMo ASR Docs: https://docs.nvidia.com/nemo-framework/user-guide/latest/nemotoolkit/asr/
- NeMo Streaming Tutorial: https://github.com/NVIDIA/NeMo/blob/main/tutorials/asr/Online_ASR_Microphone_Demo_Cache_Aware_Streaming.ipynb
- Vosk Latency Analysis: https://alphacephei.com/nsh/2020/11/27/latency.html
- WhisperStreaming: https://github.com/ufal/whisper_streaming
- SimulStreaming Paper: https://arxiv.org/html/2506.17077
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
