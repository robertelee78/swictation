# Swictation Streaming Pipeline Architecture

## Overview

Swictation uses a **hotkey-toggled streaming pipeline** with no Voice Activity Detection (VAD) for the MVP. The user explicitly controls when dictation is active via `Super+Shift+D`.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                  SWICTATION DAEMON (systemd)                │
│                                                             │
│  State Machine: [IDLE] ↔ [RECORDING]                      │
│  IPC: D-Bus socket for toggle commands                    │
└─────────────────────────────────────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
    ┌──────────────┐ ┌──────────┐ ┌──────────────┐
    │  D-Bus IPC   │ │  Config  │ │  PipeWire    │
    │   (toggle)   │ │  (TOML)  │ │  (audio src) │
    └──────┬───────┘ └──────────┘ └──────┬───────┘
           │                              │
           │ Super+Shift+D                ▼
           │                    ┌──────────────────┐
           │                    │  Audio Capture   │
           │                    │  - 16kHz mono    │
           │                    │  - Circular buf  │
           │                    │  - 100ms chunks  │
           │                    └─────────┬────────┘
           │                              │
           └──────────► [TOGGLE ON] ──────┤
                                          ▼
                            ┌──────────────────────────┐
                            │  Canary-1B-Flash STT     │
                            │  - GPU accelerated       │
                            │  - Streaming inference   │
                            │  - RTFx: 749-1000x      │
                            │  - Latency: <80ms       │
                            └─────────┬────────────────┘
                                      │
                                      ▼
                            ┌──────────────────────────┐
                            │  Midstream Pipeline      │
                            │  - Punctuation restore   │
                            │  - Code transforms       │
                            │  - Middleware chain      │
                            │  - Latency: <10ms       │
                            └─────────┬────────────────┘
                                      │
                                      ▼
                            ┌──────────────────────────┐
                            │   wtype Injection        │
                            │  - Char-by-char stream   │
                            │  - Unicode support       │
                            │  - Latency: <20ms       │
                            └──────────────────────────┘
```

## State Machine

```
┌─────────┐
│  IDLE   │
└────┬────┘
     │ Super+Shift+D pressed
     ▼
┌─────────────┐
│  RECORDING  │────────► Audio stream active
└─────┬───────┘          STT processing
      │                  Text injection
      │ Super+Shift+D pressed again
      ▼
┌─────────┐
│  IDLE   │
└─────────┘
```

## Component Specifications

### 1. PipeWire Audio Capture

**Purpose**: Capture microphone audio in real-time

**Specifications**:
- **Sample rate**: 16kHz (Canary native format)
- **Channels**: Mono (1 channel)
- **Bit depth**: 16-bit signed PCM
- **Buffer size**: 1600 samples (100ms at 16kHz)
- **Circular buffer**: 10 chunks (1 second lookahead)

**Implementation**:
- Use `sounddevice` Python library
- Non-blocking callback-based capture
- Thread-safe ring buffer

**Latency target**: <5ms

### 2. NVIDIA Canary-1B-Flash STT Engine

**Purpose**: Convert speech audio to text in real-time

**Model specs**:
- **Model**: Canary-1B-Flash
- **Size**: ~1GB
- **VRAM usage**: ~1.5GB during inference
- **Accuracy**: 5.77% WER
- **Speed**: 749-1000 RTFx

**Streaming configuration**:
- **Chunk duration**: 100ms audio chunks
- **Inference batch size**: 1 (real-time)
- **GPU device**: CUDA device 0 (RTX A1000)
- **Compute dtype**: float16 (for speed)

**Latency target**: <80ms

### 3. Midstream Text Processing Pipeline

**Purpose**: Transform raw transcription for code dictation use case

**Middleware chain**:
1. **Punctuation restoration** (if Canary doesn't provide)
2. **Code-specific transforms**:
   - "new line" → "\n"
   - "tab" → "\t"
   - "semicolon" → ";"
   - "open brace" → "{"
   - "close brace" → "}"
   - "equals" → "="
   - "plus" → "+"
   - etc.
3. **Capitalization rules**:
   - Sentence start
   - After period
   - "I" → "I"
4. **Number expansion**:
   - "one two three" → "123" (optional)

**Implementation**:
- Use midstream reactive middleware library
- Chain transformations as async functions
- Maintain context for multi-word commands

**Latency target**: <10ms

### 4. wtype Text Injection

**Purpose**: Inject transcribed text into active application

**Features**:
- Character-by-character streaming
- Unicode support (UTF-8)
- Special character handling
- Wayland-native (no X11 dependencies)

**Fallback**: Clipboard injection if wtype fails

**Latency target**: <20ms per character batch

## Threading Model

**Multi-threaded design for low latency**:

```
┌─────────────────┐
│  Main Thread    │
│  - D-Bus IPC    │
│  - State mgmt   │
└────────┬────────┘
         │
         ├──────────────┐
         ▼              ▼
┌─────────────┐  ┌──────────────┐
│ Audio Thread│  │  STT Thread  │
│  - Capture  │  │  - Canary    │
│  - Buffer   │  │  - Inference │
└──────┬──────┘  └──────┬───────┘
       │                │
       │  Queue         │  Queue
       └────────┬───────┘
                ▼
        ┌───────────────┐
        │ Process Thread│
        │  - Midstream  │
        │  - Injection  │
        └───────────────┘
```

**Thread communication**:
- **Audio → STT**: Lock-free ring buffer (SPSC queue)
- **STT → Process**: Thread-safe queue with backpressure
- **Process → Injection**: Async task queue

## Performance Budget

**End-to-end latency target: <150ms**

| Component | Target Latency | Notes |
|-----------|----------------|-------|
| Audio capture | <5ms | Hardware + buffer overhead |
| Audio → STT queue | <5ms | Memory copy |
| Canary inference | <80ms | GPU inference on 100ms chunk |
| STT → Process queue | <5ms | Memory copy |
| Midstream processing | <10ms | Text transformations |
| wtype injection | <20ms | System call + rendering |
| **TOTAL** | **<125ms** | ✅ Under 150ms target |

## Memory Footprint

**Estimated memory usage**:

| Component | Memory |
|-----------|--------|
| Python runtime | ~50MB |
| sounddevice + deps | ~20MB |
| NVIDIA NeMo framework | ~200MB |
| Canary-1B-Flash model | ~1.0GB |
| Audio buffers | ~2MB |
| GPU VRAM | ~1.5GB |
| **Total RAM** | **~1.3GB** |
| **Total VRAM** | **~1.5GB** |

**Remaining headroom on RTX A1000**: ~2.5GB VRAM ✅

## Error Handling & Edge Cases

### Audio Capture Failures
- **Detection**: Monitor sounddevice errors
- **Recovery**: Restart capture thread
- **User feedback**: Notification via notify-send

### GPU OOM (Out of Memory)
- **Detection**: CUDA OOM exception
- **Recovery**: Fallback to CPU inference (slower)
- **User feedback**: Warning notification

### wtype Injection Failure
- **Detection**: Non-zero exit code
- **Fallback**: Clipboard injection
- **User feedback**: Console warning

### Hotkey Toggle Race Conditions
- **Prevention**: State lock on toggle operations
- **Behavior**: Ignore rapid toggle spam (<200ms)

## Configuration

**Config file**: `~/.config/swictation/config.toml`

```toml
[audio]
sample_rate = 16000
channels = 1
chunk_duration_ms = 100
device = "default"  # or specific PipeWire device

[stt]
model = "nvidia/canary-1b-flash"
device = "cuda"  # or "cpu"
compute_dtype = "float16"

[processing]
enable_code_transforms = true
enable_punctuation = true
enable_capitalization = true

[injection]
method = "wtype"  # or "clipboard"
char_delay_ms = 0  # optional delay between chars

[keybinding]
toggle_key = "mod1+Shift+d"
notification_enabled = true
```

## Risk Analysis

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| GPU driver issues | High | Low | Comprehensive error handling, CPU fallback |
| Audio device detection | Medium | Medium | Device enumeration + config override |
| STT latency >200ms | High | Low | Optimized chunk size, GPU acceleration |
| Text injection failures | Medium | Low | Clipboard fallback |
| Memory leaks | Medium | Medium | Proper resource cleanup, monitoring |

## Future Enhancements (Phase 2)

1. **Optional VAD**: Skip silence periods for battery optimization
2. **Custom vocabulary**: User-defined code snippets
3. **Voice commands**: "undo last", "delete line", etc.
4. **Multi-language**: Beyond English
5. **Model hot-swapping**: Switch models without restart

## References

- NVIDIA NeMo: https://github.com/NVIDIA-NeMo/NeMo
- Canary-1B-Flash: https://huggingface.co/nvidia/canary-1b-flash
- Midstream: https://www.npmjs.com/package/midstream
- PipeWire: https://pipewire.org/
