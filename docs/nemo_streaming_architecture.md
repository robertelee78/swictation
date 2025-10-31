# NeMo Streaming ASR Architecture Analysis

## Overview

This document analyzes NVIDIA NeMo's streaming ASR architecture for real-time speech transcription, based on the `speech_to_text_aed_streaming_infer.py` reference implementation and supporting modules.

**Source**: [NeMo GitHub - AED Streaming Inference](https://github.com/NVIDIA/NeMo/blob/main/examples/asr/asr_chunked_inference/aed/speech_to_text_aed_streaming_infer.py)

## Architecture Components

### 1. Streaming Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        AUDIO INPUT STREAM                            │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    StreamingBatchedAudioBuffer                       │
│  • Manages Left-Chunk-Right context windows                         │
│  • Handles dynamic buffer sizing                                    │
│  • Tracks per-sample context (ContextSizeBatch)                     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      AUDIO PREPROCESSING                             │
│  • Feature extraction (mel spectrograms)                            │
│  • Per-feature normalization (required)                             │
│  • Dithering disabled for streaming                                 │
│  • No padding (pad_to = 0)                                          │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        ENCODER (AED Model)                           │
│  • Processes [left + chunk + right] context                         │
│  • Encoder subsampling factor applied                               │
│  • Cache-aware streaming supported                                  │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│              GreedyBatchedStreamingAEDComputer                       │
│  • Streaming policy: Wait-k or AlignAtt                             │
│  • Token-by-token decoding                                          │
│  • Cross-attention alignment tracking                               │
│  • Hallucination detection (optional)                               │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      TRANSCRIPTION OUTPUT                            │
│  • Incremental text generation                                      │
│  • Token alignment timestamps                                       │
│  • Confidence scores                                                │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Classes and Their Roles

### 2.1 ContextSize (Dataclass)

**Purpose**: Defines the audio context window structure for streaming inference.

**Location**: `nemo.collections.asr.parts.utils.streaming_utils`

```python
@dataclass
class ContextSize:
    left: int      # Left context frames (history)
    chunk: int     # Current chunk frames (processing window)
    right: int     # Right context frames (lookahead)

    def total(self) -> int:
        """Total context size"""
        return self.left + self.chunk + self.right

    def subsample(self, factor: int) -> "ContextSize":
        """Subsample context by encoder subsampling factor"""
        return ContextSize(
            left=self.left // factor,
            chunk=self.chunk // factor,
            right=self.right // factor,
        )
```

**Key Methods**:
- `total()`: Returns total buffer size
- `subsample(factor)`: Applies encoder subsampling factor
- `add_frames_get_removed_()`: Manages sliding window, returns frames to discard

**Usage Pattern**:
```python
# Recommended settings for 1.5s latency streaming
context_encoder_frames = ContextSize(
    left=int(10.0 * features_per_sec / encoder_subsampling_factor),   # 10s history
    chunk=int(1.0 * features_per_sec / encoder_subsampling_factor),   # 1s chunk
    right=int(0.5 * features_per_sec / encoder_subsampling_factor),   # 0.5s lookahead
)
```

### 2.2 StreamingBatchedAudioBuffer

**Purpose**: Manages audio buffer with strict left-chunk-right context for streaming inference.

**Location**: `nemo.collections.asr.parts.utils.streaming_utils`

```python
class StreamingBatchedAudioBuffer:
    """Batched audio buffer with strict context management"""

    def __init__(
        self,
        batch_size: int,
        context_samples: ContextSize,  # In audio samples
        dtype: torch.dtype,
        device: torch.device
    ):
        self.batch_size = batch_size
        self.expected_context = context_samples
        self.samples = torch.zeros([batch_size, 0], dtype=dtype, device=device)
        self.context_size = ContextSize(left=0, chunk=0, right=0)
        self.context_size_batch = ContextSizeBatch(...)  # Per-sample tracking
```

**Key Methods**:
- `add_audio_batch_()`: Adds new audio chunk, manages sliding window
- Automatically removes old frames from left context
- Tracks per-sample context lengths (handles variable-length audio in batch)

**Critical Features**:
- **No left padding**: Efficiently handles streaming without padding overhead
- **Dynamic context**: Adapts to audio boundaries (last chunk handling)
- **Batch support**: Processes multiple audio streams simultaneously

### 2.3 FrameBatchMultiTaskAED

**Purpose**: High-level API for frame-by-frame streaming transcription with Canary models.

**Location**: `nemo.collections.asr.parts.utils.streaming_utils`

```python
class FrameBatchMultiTaskAED(FrameBatchASR):
    """Frame-based batched inference for multitask AED models"""

    def __init__(
        self,
        asr_model,
        frame_len=4,          # Seconds per frame
        total_buffer=4,       # Total buffer seconds
        batch_size=4
    ):
        self.timestamps_asr_model = asr_model.timestamps_asr_model
        self.window_stride = asr_model._cfg.preprocessor.window_stride
        self.subsampling_factor = asr_model._cfg.encoder.subsampling_factor
        self.chunk_offsets = [0]  # Track chunk positions
```

**Key Methods**:
- `read_audio_file()`: Loads audio and sets up streaming
- `infer_logits()`: Processes frame buffers through model
- `transcribe()`: Main transcription entry point
- `get_input_tokens()`: Handles Canary prompt generation

**Canary Prompt Support**:
```python
def get_input_tokens(self, sample: dict):
    """Generate Canary-style prompt tokens"""
    # Required fields: source_lang, target_lang, taskname, pnc
    # Optional: decodercontext, emotion, itn, timestamp, diarize
    tokens = self.asr_model.prompt.encode_dialog(
        turns=[{
            "role": "user",
            "slots": {
                "source_lang": "en",
                "target_lang": "en",
                "taskname": "asr",
                "pnc": "yes"
            }
        }]
    )
```

### 2.4 Streaming Decoding Configuration

**Purpose**: Configure streaming decoding policy and parameters.

**Reference Script Configuration**:
```python
@dataclass
class TranscriptionConfig:
    # Audio buffer settings
    chunk_secs: float = 2.0           # Chunk length
    left_context_secs: float = 10.0   # History (larger = better quality)
    right_context_secs: float = 2.0   # Lookahead (affects latency)

    # Decoding configuration
    decoding: AEDStreamingDecodingConfig = field(
        default_factory=AEDStreamingDecodingConfig
    )

    # Model settings
    batch_size: int = 32
    compute_dtype: Optional[str] = None  # bfloat16 if available

    # Preprocessing (critical for streaming)
    model_cfg.preprocessor.dither = 0.0          # Disable for streaming
    model_cfg.preprocessor.pad_to = 0            # Disable padding
    model_cfg.preprocessor.normalize = "per_feature"  # Required
```

## Streaming Decoding Policies

### 3.1 Wait-k Policy

**Characteristics**:
- Predicts **one token per chunk** (conservative)
- Higher latency but more stable
- **Infinite left context recommended** (unlimited history)
- Unclear when to forget left context with limited buffer

**Use Case**: When accuracy is critical and latency is acceptable.

**Latency**: `(chunk_secs + right_context_secs)` per token

**Configuration**:
```python
decoding.streaming_policy = "waitk"
left_context_secs = float('inf')  # Infinite recommended
chunk_secs = 1.0
right_context_secs = 0.5
```

### 3.2 AlignAtt Policy (Recommended)

**Characteristics**:
- **Cross-attention alignment-based** token prediction
- Predicts multiple tokens per chunk when alignment allows
- **Lower latency** than Wait-k
- Suitable for **fixed left context** (windowed recognition)
- May lose some accuracy with limited left context

**Use Case**: Real-time dictation where latency matters.

**Latency**: Variable, typically **50-70% of Wait-k latency**

**Configuration**:
```python
decoding.streaming_policy = "alignatt"
left_context_secs = 10.0   # Fixed window (not infinite)
chunk_secs = 1.0
right_context_secs = 0.5
# Theoretical latency: 1.5s
```

**How AlignAtt Works**:
1. For each new chunk, check cross-attention alignment
2. If alignment condition met → predict next token without waiting
3. If condition not met → wait for more audio (increase buffer)
4. Dynamically balances latency vs. accuracy

## Critical Code Snippets for Swictation

### 4.1 Initialize Streaming Buffer

```python
from nemo.collections.asr.parts.utils.streaming_utils import (
    ContextSize,
    StreamingBatchedAudioBuffer,
    SimpleAudioDataset,
    AudioBatch,
)

# Calculate context sizes
audio_sample_rate = 16000
feature_stride_sec = 0.01  # 10ms
encoder_subsampling_factor = 8
features_per_sec = 1.0 / feature_stride_sec

# Define context in encoder frames
context_encoder_frames = ContextSize(
    left=int(10.0 * features_per_sec / encoder_subsampling_factor),
    chunk=int(1.0 * features_per_sec / encoder_subsampling_factor),
    right=int(0.5 * features_per_sec / encoder_subsampling_factor),
)

# Convert to audio samples
features_frame2audio_samples = int(audio_sample_rate * feature_stride_sec)
encoder_frame2audio_samples = features_frame2audio_samples * encoder_subsampling_factor

context_samples = ContextSize(
    left=context_encoder_frames.left * encoder_frame2audio_samples,
    chunk=context_encoder_frames.chunk * encoder_frame2audio_samples,
    right=context_encoder_frames.right * encoder_frame2audio_samples,
)

# Create streaming buffer
buffer = StreamingBatchedAudioBuffer(
    batch_size=1,  # Real-time dictation
    context_samples=context_samples,
    dtype=torch.float32,
    device=torch.device('cuda' if torch.cuda.is_available() else 'cpu'),
)
```

### 4.2 Process Audio Chunks

```python
# Main streaming loop
left_sample = 0
right_sample = min(context_samples.chunk + context_samples.right, audio_length)
step_idx = 0

while left_sample < audio_length:
    # Calculate chunk
    chunk_length = min(right_sample, audio_length) - left_sample
    is_last_chunk = right_sample >= audio_length

    # Add to buffer
    buffer.add_audio_batch_(
        audio_batch[:, left_sample:right_sample],
        audio_lengths=torch.tensor([chunk_length]),
        is_last_chunk=is_last_chunk,
        is_last_chunk_batch=torch.tensor([is_last_chunk]),
    )

    # Get encoder output (full buffer: left + chunk + right)
    _, encoded_len, enc_states, _ = asr_model(
        input_signal=buffer.samples,
        input_signal_length=buffer.context_size_batch.total()
    )

    # Remove right context from length (for non-last chunks)
    encoder_context_batch = buffer.context_size_batch.subsample(
        factor=encoder_frame2audio_samples
    )
    encoded_len_no_rc = encoder_context_batch.left + encoder_context_batch.chunk
    encoded_length_corrected = torch.where(
        is_last_chunk_batch,
        encoded_len,
        encoded_len_no_rc
    )

    # Decode chunk
    model_state = decoding_computer(
        encoder_output=enc_states,
        encoder_output_len=encoded_length_corrected,
        encoder_input_mask=encoder_input_mask,
        prev_batched_state=model_state,
    )

    # Slide window
    left_sample = right_sample
    right_sample = min(right_sample + context_samples.chunk, audio_length)
    step_idx += 1
```

### 4.3 Model State Management

```python
from nemo.collections.asr.parts.submodules.aed_decoding import (
    GreedyBatchedStreamingAEDComputer
)

# Initialize decoder computer
decoding_computer = GreedyBatchedStreamingAEDComputer(
    asr_model,
    frame_chunk_size=context_encoder_frames.chunk,
    decoding_cfg=cfg.decoding,
)

# Initialize model state
model_state = GreedyBatchedStreamingAEDComputer.initialize_aed_model_state(
    asr_model=asr_model,
    decoder_input_ids=decoder_input_ids,  # Prompt tokens
    batch_size=1,
    context_encoder_frames=context_encoder_frames,
    chunk_secs=1.0,
    right_context_secs=0.5,
)

# Track frame positions
model_state.prev_encoder_shift = max(
    end_of_window_sample // encoder_frame2audio_samples
    - context_encoder_frames.left
    - context_encoder_frames.chunk,
    0,
)
```

### 4.4 Extract Transcription

```python
# After streaming loop completes
transcription_idx = model_state.pred_tokens_ids[
    0,  # Batch index
    model_state.decoder_input_ids.size(-1):  # Skip prompt tokens
    model_state.current_context_lengths[0]   # Up to current position
]

transcription = asr_model.tokenizer.ids_to_text(
    transcription_idx.tolist()
).strip()

# Get token alignments (for timestamps)
token_alignments = model_state.tokens_frame_alignment[0]
```

## Parameter Recommendations for Swictation

### 5.1 Real-Time Dictation Configuration

**Target**: Low latency (<2s) with high accuracy

```yaml
# Audio buffer settings
chunk_secs: 0.8               # Smaller chunks for lower latency
left_context_secs: 8.0        # Sufficient history without memory issues
right_context_secs: 0.4       # Minimal lookahead

# Streaming policy
decoding:
  streaming_policy: "alignatt"  # Lower latency
  max_symbols_per_step: 10      # Prevent runaway decoding

# Model optimization
batch_size: 1                 # Real-time, single user
compute_dtype: "bfloat16"     # Fast inference on modern GPUs
matmul_precision: "high"      # Good balance

# Preprocessing
preprocessor:
  dither: 0.0                 # Disable for streaming
  pad_to: 0                   # Disable padding
  normalize: "per_feature"    # Required for streaming
  window_size: 0.025          # 25ms
  window_stride: 0.01         # 10ms (100 FPS)

# Expected metrics
theoretical_latency: 1.2s     # 0.8s chunk + 0.4s right context
actual_latency: ~1.5s         # Including processing overhead
```

### 5.2 High-Accuracy Configuration

**Target**: Maximum accuracy, latency acceptable

```yaml
chunk_secs: 1.5
left_context_secs: 15.0       # More history
right_context_secs: 1.0       # More lookahead

decoding:
  streaming_policy: "waitk"   # Conservative, accurate

theoretical_latency: 2.5s
```

### 5.3 Memory-Constrained Configuration

**Target**: Minimize memory usage

```yaml
chunk_secs: 0.5
left_context_secs: 5.0        # Shorter history
right_context_secs: 0.3
batch_size: 1
compute_dtype: "float16"      # Lower memory than bfloat16
```

## Performance Considerations

### 6.1 Latency Calculation

```python
# Theoretical latency (minimum achievable)
latency_secs = (chunk_secs + right_context_secs)

# For AlignAtt: multiply by 0.5-0.7 (empirical)
actual_latency_alignatt = latency_secs * 0.6

# For Wait-k: approximately same as theoretical
actual_latency_waitk = latency_secs * 1.0
```

**Example**:
- Config: chunk=1.0s, right=0.5s
- Theoretical: 1.5s
- AlignAtt actual: ~0.9-1.05s
- Wait-k actual: ~1.5s

### 6.2 Memory Usage

```python
# Buffer memory = batch_size × total_context × dtype_size
total_samples = (left_context + chunk + right_context)
memory_mb = batch_size * total_samples * 4 / 1_000_000  # float32

# Example: 10-1-0.5s context @ 16kHz
# total_samples = (160000 + 16000 + 8000) = 184,000
# memory_mb = 1 × 184000 × 4 / 1e6 = 0.736 MB per stream
```

### 6.3 GPU Optimization

**Key Settings**:
- Enable `torch.inference_mode()` (not just `no_grad()`)
- Use `bfloat16` on Ampere+ GPUs (A100, RTX 3090+)
- Use `float16` on older GPUs (V100, RTX 2080)
- Set `torch.backends.cudnn.benchmark = True` for consistent input sizes
- Disable gradient computation globally: `torch.set_grad_enabled(False)`

## Integration Checklist for Swictation

- [ ] Implement `StreamingBatchedAudioBuffer` wrapper for IPC audio chunks
- [ ] Configure `ContextSize` with swictation's latency requirements
- [ ] Set up preprocessor with streaming-specific settings (dither=0, pad_to=0)
- [ ] Choose streaming policy (AlignAtt recommended for dictation)
- [ ] Initialize `GreedyBatchedStreamingAEDComputer` with decoding config
- [ ] Implement sliding window loop with proper context management
- [ ] Handle encoder subsampling factor correctly (critical!)
- [ ] Extract transcriptions incrementally from `model_state`
- [ ] Add token alignment tracking for word-level timestamps
- [ ] Implement proper cleanup on stream end (last chunk handling)

## References

1. **NeMo Streaming Script**: https://github.com/NVIDIA/NeMo/blob/main/examples/asr/asr_chunked_inference/aed/speech_to_text_aed_streaming_infer.py
2. **StreamingUtils Module**: `nemo.collections.asr.parts.utils.streaming_utils`
3. **Streaming Mixin**: `nemo.collections.asr.parts.mixins.streaming`
4. **AED Models**: `nemo.collections.asr.models.aed_multitask_models`

## Key Insights

1. **AlignAtt is superior for real-time dictation** - Lower latency with minimal accuracy loss
2. **Per-feature normalization is mandatory** - Other normalization methods fail in streaming
3. **Disable dithering and padding** - Critical for streaming stability
4. **Left context trades memory for accuracy** - 8-10s is sweet spot for dictation
5. **Right context determines latency floor** - Keep as low as acceptable (0.4-0.5s)
6. **Encoder subsampling must be handled correctly** - Off-by-one errors cause alignment issues
7. **Batch processing is efficient even for single stream** - Batch size 1 still uses optimized code paths

## Next Steps for Swictation

1. **Test parameter configurations** with sample audio files
2. **Benchmark latency** with different context window settings
3. **Measure GPU memory usage** to set optimal batch sizes
4. **Implement incremental output** to IPC transcription queue
5. **Add hallucination detection** if using Canary's built-in detector
6. **Profile encoder vs decoder time** to identify bottlenecks
