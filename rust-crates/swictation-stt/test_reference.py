#!/usr/bin/env python3
"""Get reference transcription using sherpa-onnx Python"""

import sherpa_onnx
import wave

# Load model
recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
    encoder="/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx",
    decoder="/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/decoder.int8.onnx",
    joiner="/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/joiner.int8.onnx",
    tokens="/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/tokens.txt",
    num_threads=4,
    sample_rate=16000,
    feature_dim=128,
)

# Load audio
wav_file = "/opt/swictation/models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/test_wavs/en_16k.wav"
with wave.open(wav_file, 'rb') as wf:
    sample_rate = wf.getframerate()
    n_channels = wf.getnchannels()
    n_frames = wf.getnframes()
    audio_data = wf.readframes(n_frames)

# Convert to float32
import numpy as np
samples = np.frombuffer(audio_data, dtype=np.int16).astype(np.float32) / 32768.0

# Recognize
stream = recognizer.create_stream()
stream.accept_waveform(sample_rate, samples)
recognizer.decode_stream(stream)
result = stream.result

print(f"Reference transcription: '{result.text}'")
print(f"Duration: {len(samples)/sample_rate:.2f}s")
