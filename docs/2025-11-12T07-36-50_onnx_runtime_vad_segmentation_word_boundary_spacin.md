# Search Results

**Query:** ONNX Runtime VAD segmentation word boundary spacing ASR
**Date:** 2025-11-12T07:36:50.914Z

## Answer

**ONNX Runtime-based Voice Activity Detection (VAD) models segment audio by detecting speech regions, but the spacing of word boundaries in ASR pipelines depends on both VAD segmentation accuracy and subsequent ASR alignment methods.** VAD models typically output timestamps for speech segments, which are then used to guide ASR processing, but these segments do not always correspond precisely to word boundaries, and additional alignment or post-processing is often required for accurate word-level timestamps.

---

### 1. **VAD Segmentation with ONNX Runtime**

- **ONNX Runtime** is widely used for deploying VAD models such as Silero, FSMN, and PyAnnote for efficient, real-time speech segmentation[1][3][4].
- VAD models process raw audio and output **speech segment timestamps**, e.g., start and end times for detected speech regions[3].
- Example output:
  ```
  Timestamps in Second:
  00:00:00.500 --> 00:00:02.500
  00:00:04.200 --> 00:00:05.599
  ```
  These segments are typically based on silence detection and do not guarantee alignment with word boundaries[3].

---

### 2. **Word Boundary Spacing in ASR Pipelines**

- **VAD segments** are used to split long audio into manageable chunks for ASR models, improving recognition efficiency and latency[7].
- However, **VAD segmentation is based on acoustic features (speech/non-speech)**, not linguistic content, so segment boundaries may fall within or between words, especially in spontaneous speech with hesitations or short pauses[7].
- This can lead to **suboptimal word boundary placement**:
  - Example: If a speaker pauses after "Shaq..." in "Shaq... dunks—game over!", VAD may segment after "Shaq", but the ideal segmentation for ASR would keep "Shaq dunks" together for better context and recognition[7].

---

### 3. **Improving Word Boundary Accuracy**

- **Post-processing and alignment** are required to obtain accurate word-level timestamps from VAD-segmented audio:
  - Some ASR models (e.g., Whisper, Vosk, Nemo) can output **token or word-level timestamps** using forced alignment or duration modeling.
  - Recent research proposes models that estimate word boundaries directly from ASR encoder embeddings, without relying on phonetic lexicons, improving scalability and accuracy for multilingual ASR.
- **Joint segmentation and decoding** approaches (E2E segmenters) can outperform VAD-only segmentation by conditioning boundary decisions on both acoustic and semantic features, reducing word error rates and improving segment latency[7].

---

### 4. **Practical Considerations for ONNX Runtime VAD + ASR**

- **VAD segment length and spacing** are typically controlled by silence duration thresholds (e.g., 200 ms of silence triggers a segment boundary)[3][7].
- **Word boundary accuracy** depends on:
  - VAD model sensitivity (thresholds, window size)
  - ASR model's ability to align recognized words to audio frames (forced alignment, token timestamping)
  - Post-processing to refine segment boundaries to match word boundaries, if needed.
- **ONNX ASR toolkits** (e.g., sherpa-onnx, onnx-asr) support combining VAD and ASR, and some offer experimental features for longform recognition with VAD and token timestamp output[6].

---

### 5. **Summary Table: VAD vs. ASR Word Boundary Handling**

| Component         | Output Type                | Boundary Spacing Control         | Word-Level Accuracy |
|-------------------|---------------------------|----------------------------------|---------------------|
| VAD (ONNX models) | Speech segment timestamps | Silence threshold, window size   | Low (not word-aware)|
| ASR (ONNX models) | Transcription, timestamps | Forced alignment, token mapping  | High (if supported) |
| Joint E2E models  | Segments + transcription  | Acoustic + semantic features     | Highest             |

---

**In conclusion:**  
- **ONNX Runtime VAD models segment audio by detecting speech regions, but do not inherently provide word boundaries.**
- **Accurate word boundary spacing in ASR requires additional alignment, either via forced alignment, token timestamping, or joint segmentation-decoding models.**
- **For best results, use VAD for coarse segmentation and ASR models with word-level timestamping or post-processing for precise word boundaries.**[7]

If you need implementation details or recommendations for specific ONNX models or toolkits supporting word-level timestamps, please specify your ASR framework or use case.

## Citations

### 1. GitHub - dangvansam/pyannote-onnx: PyAnnote Voice Activity Detection (ONNX version)

**URL:** https://github.com/dangvansam/pyannote-onnx

dangvansam / **
pyannote-onnx ** Public
forked from pyannote/pyannote-audio

# dangvansam/pyannote-onnx

dangvansam

becb36b · Sep 8, 2023... # Voice Activity Detection with pyannote.audio (ONNX version)

Suported ONNX runtime for pyannote.audio

## Installation

Only Python 3.8+ is supported.

```
# for CPU
pip install onnxruntime
# for GPU, check version on: https://onnxruntime.ai/docs/build/eps.html#cuda
pip install onnxruntime-gpu
# install pyannote
pip install -e .
```

## 1. Export ONNX from PyTorch model

```
# 1. Download pytorch model (.bin) from https://huggingface.co/pyannote/segmentation/blob/main/pytorch_model.bin
wget https://huggingface.co/pyannote/segmentation/blob/main/pytorch_model.bin -O pytorch_model/vad_model.bin
# 2. Export
python onnx/export_onnx.py -i pytorch_model/vad_model.bin -o onnx_model/

```

## Run VAD

```
# use onnx model (2x faster)
python vad.py -m onnx_model/vad_model.onnx -i tests/data/test_vad.wav
# mean time cost = 5.32921104

# use pytorch model
python vad.py -m onnx_model/vad_model.bin -i tests/data/test_vad.wav
# mean time cost = 9.56711404
```... ## Benchmark

Test file tests/data/test_vad.wav with duration 6m15s

- CPU Intel(R) Xeon(R) CPU E5-2683 v3 @ 2.00GHz
- GPU Nvidia GTX 1080Ti

Batch size 32

|Backend|CPU time (s)|GPU time (s)|
|--|--|--|
|PyTorch|12.0|1.5|
|ONNX|4.33|NA|

Batch size 64

|Backend|CPU time (s)|GPU time (s)|
|--|--|--|
|PyTorch|inf|1.99|
|ONNX|4.02|NA|

## Citations

If you use
```
pyannote.audio
```
please use the following citations:

```
@inproceedings{Bredin2020,
  Title = {{pyannote.audio: neural building blocks for speaker diarization}},
  Author = {{Bredin}, Herv{\'e} and {Yin}, Ruiqing and {Coria}, Juan Manuel and {Gelly}, Gregory and {Korshunov}, Pavel and {Lavechin}, Marvin and {Fustes}, Diego and {Titeux}, Hadrien and {Bouaziz}, Wassim and {Gill}, Marie-Philippe},
  Booktitle = {ICASSP 2020, IEEE International Conference on Acoustics, Speech, and Signal Processing},
  Year = {2020},
}
```

```
@inproceedings{Bredin2021,
  Title = {{End-to-end speaker segmentation for overlap-aware resegmentation}},
  Author = {{Bredin}, Herv{\'e} and {Laurent}, Antoine},
  Booktitle = {Proc. Interspeech 2021},
  Year = {2021},
}
```... ## About

PyAnnote Voice Activity Detection (ONNX version)

### Topics

vad audio-segmentation speech-separation onnx speech-activity-detection audio-split audio-splitter pyannote voice-ac

### Resources

Readme

### License

MIT license

### Uh oh!

There was an error while loading. Please reload this page.

Activity

### Stars

**17** stars

### Watchers

**0** watching

### Forks

**5** forks

## Languages

- Jupyter Notebook 61.4%
- Python 27.2%
- JavaScript 11.2%
- Other 0.2%

### 2. GitHub - k2-fsa/sherpa-onnx: Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

**URL:** http://github.com/k2-fsa/sherpa-onnx

# k2-fsa/sherpa-onnx

### Supported functions

|Speech recognition|Speech synthesis|Source separation|
|--|--|--|
|✔️|✔️|✔️|
|Speaker identification|Speaker diarization|Speaker verification|
|--|--|--|
|✔️|✔️|✔️|
|Spoken Language identification|Audio tagging|Voice activity detection|
|--|--|--|
|✔️|✔️|✔️|
|Keyword spotting|Add punctuation|Speech enhancement|
|--|--|--|
|✔️|✔️|✔️|

### Supported platforms

|Architecture|Android|iOS|Windows|macOS|linux|HarmonyOS|
|--|--|--|--|--|--|--|
|x64|✔️| |✔️|✔️|✔️|✔️|
|x86|✔️| |✔️| | | |
|arm64|✔️|✔️|✔️|✔️|✔️|✔️|
|arm32|✔️| | | |✔️|✔️|
|riscv64| | | | |✔️| |

### Supported programming languages

|1. C++|2. C|3. Python|4. JavaScript|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|5. Java|6. C#|7. Kotlin|8. Swift|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|9. Go|10. Dart|11. Rust|12. Pascal|
|--|--|--|--|
|✔️|✔️|✔️|✔️|

For Rust support, please see sherpa-rs

It also supports WebAssembly.... ## Introduction

This repository supports running the following functions **locally**

- Speech-to-text (i.e., ASR); both streaming and non-streaming are supported
- Text-to-speech (i.e., TTS)
- Speaker diarization
- Speaker identification
- Speaker verification
- Spoken language identification
- Audio tagging
- VAD (e.g., silero-vad)
- Speech enhancement (e.g., gtcrn)
- Keyword spotting
- Source separation (e.g., spleeter, UVR)

on the following platforms and operating systems:

- x86,
  ```
  x86_64
  ```
  , 32-bit ARM, 64-bit ARM (arm64, aarch64), RISC-V (riscv64), **RK NPU**
- Linux, macOS, Windows, openKylin
- Android, WearOS
- iOS
- HarmonyOS
- NodeJS
- WebAssembly
- NVIDIA Jetson Orin NX (Support running on both CPU and GPU)
- NVIDIA Jetson Nano B01 (Support running on both CPU and GPU)
- Raspberry Pi
- RV1126
- LicheePi4A
- VisionFive 2
- 旭日X3派
- 爱芯派
- RK3588
- etc

with the following APIs

- C++, C, Python, Go,
  ```
  C#
  ```
- Java, Kotlin, JavaScript
- Swift, Rust
- Dart, Object Pascal... |Description|Huggingface space|ModelScope space|
|--|--|--|
|Voice activity detection with silero-vad|Click me|地址|
|Real-time speech recognition (Chinese + English) with Zipformer|Click me|地址|
|Real-time speech recognition (Chinese + English) with Paraformer|Click me|地址|
|Real-time speech recognition (Chinese + English + Cantonese) with Paraformer-large|Click me|地址|
|Real-time speech recognition (English)|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer CTC|Click me|地址|
|VAD + speech recognition (Chinese + English + Korean + Japanese + Cantonese) with SenseVoice|Click me|地址|
|VAD + speech recognition (English) with Whisper tiny.en|Click me|地址|
|VAD + speech recognition (English) with Moonshine tiny|Click me|地址|
|VAD + speech recognition (English) with Zipformer trained with GigaSpeech|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer trained with WenetSpeech|Click me|地址|
|VAD + speech recognition (Japanese) with Zipformer trained with ReazonSpeech|Click me|地址|
|VAD + speech recognition (Thai) with Zipformer trained with GigaSpeech2|Click me|地址|
|VAD + speech recognition (Chinese 多种方言) with a TeleSpeech-ASR CTC model|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-large|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-small|Click me|地址|
|VAD + speech recognition (多语种及多种中文方言) with Dolphin-base|Click me|地址|
|Speech synthesis (English)|Click me|地址|
|Speech synthesis (German)|Click me|地址|
|Speaker diarization|Click me|地址|... ### Links for pre-trained models

|Description|URL|
|--|--|
|Speech recognition (speech to text, ASR)|Address|
|Text-to-speech (TTS)|Address|
|VAD|Address|
|Keyword spotting|Address|
|Audio tagging|Address|
|Speaker identification (Speaker ID)|Address|
|Spoken language identification (Language ID)|See multi-lingual Whisper ASR models from Speech recognition|
|Punctuation|Address|
|Speaker segmentation|Address|
|Speech enhancement|Address|
|Source separation|Address|

#### Some pre-trained ASR models (Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-ctc/index.html

for more models. The following table lists only **SOME** of them.

|Name|Supported Languages|Description|
|--|--|--|
|sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-small-bilingual-zh-en-2023-02-16|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-zh-14M-2023-02-23|Chinese|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-en-20M-2023-02-17|English|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-korean-2024-06-16|Korean|See also|
|sherpa-onnx-streaming-zipformer-fr-2023-04-14|French|See also|... #### Some pre-trained ASR models (Non-Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-ctc/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/telespeech/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/index.html

for more models. The following table lists only **SOME** of them.... ## About

Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

k2-fsa.github.io/sherpa/onnx/index.html

### Topics

android windows macos linux lazarus raspberry-pi ios text-to-speech csharp cpp dotnet speech-to-text aarch64 mfc risc-v object-pascal asr arm32 onnx vits

### Resources

Readme

### License

Apache-2.0 license

### Uh oh!

There was an error while loading. Please reload this page.

Activity

Custom properties

### Stars

**6.9k** stars

### Watchers

**81** watching

### Forks

**806** forks

## Releases 140

v1.12.7 Latest
Jul 27, 2025

+ 139 releases

## Used by 113
-
- - - - - - - + 105

## Contributors 140
-
- - - - - - - - - - - - - + 126 contributors

## Languages

- C++ 38.3%
- Python 16.2%
- Shell 7.6%
- Kotlin 5.3%
- JavaScript 5.0%
- Dart 4.8%
- Other 22.8%

### 3. README.md · istupakov/onnx-asr at main

**URL:** https://huggingface.co/spaces/istupakov/onnx-asr/blob/main/README.md

A newer version of the Gradio SDK is available:

`5.49.1`

metadata

```

title: ONNX ASR

emoji: 🐢

colorFrom: blue

colorTo: yellow

sdk: gradio

sdk_version: 5.42.0

app_file: app.py

pinned: true

license: mit

short_description: ASR demo using onnx-asr

tags:

- asr

- onnx

preload_from_hub:

- istupakov/gigaam-v2-onnx

- istupakov/stt_ru_fastconformer_hybrid_large_pc_onnx

- istupakov/parakeet-tdt-0.6b-v2-onnx

- istupakov/parakeet-tdt-0.6b-v3-onnx

- istupakov/whisper-base-onnx

- alphacep/vosk-model-ru

- alphacep/vosk-model-small-ru

- onnx-community/silero-vad

```

Check out the configuration reference at https://huggingface.co/docs/hub/spaces-config-reference

### 4. GitHub - k2-fsa/sherpa-onnx: Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

**URL:** http://github.com/k2-fsa/sherpa-onnx

# k2-fsa/sherpa-onnx

### Supported functions

|Speech recognition|Speech synthesis|Source separation|
|--|--|--|
|✔️|✔️|✔️|
|Speaker identification|Speaker diarization|Speaker verification|
|--|--|--|
|✔️|✔️|✔️|
|Spoken Language identification|Audio tagging|Voice activity detection|
|--|--|--|
|✔️|✔️|✔️|
|Keyword spotting|Add punctuation|Speech enhancement|
|--|--|--|
|✔️|✔️|✔️|

### Supported platforms

|Architecture|Android|iOS|Windows|macOS|linux|HarmonyOS|
|--|--|--|--|--|--|--|
|x64|✔️| |✔️|✔️|✔️|✔️|
|x86|✔️| |✔️| | | |
|arm64|✔️|✔️|✔️|✔️|✔️|✔️|
|arm32|✔️| | | |✔️|✔️|
|riscv64| | | | |✔️| |

### Supported programming languages

|1. C++|2. C|3. Python|4. JavaScript|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|5. Java|6. C#|7. Kotlin|8. Swift|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|9. Go|10. Dart|11. Rust|12. Pascal|
|--|--|--|--|
|✔️|✔️|✔️|✔️|

For Rust support, please see sherpa-rs

It also supports WebAssembly.... ## Introduction

This repository supports running the following functions **locally**

- Speech-to-text (i.e., ASR); both streaming and non-streaming are supported
- Text-to-speech (i.e., TTS)
- Speaker diarization
- Speaker identification
- Speaker verification
- Spoken language identification
- Audio tagging
- VAD (e.g., silero-vad)
- Speech enhancement (e.g., gtcrn)
- Keyword spotting
- Source separation (e.g., spleeter, UVR)

on the following platforms and operating systems:

- x86,
  ```
  x86_64
  ```
  , 32-bit ARM, 64-bit ARM (arm64, aarch64), RISC-V (riscv64), **RK NPU**
- Linux, macOS, Windows, openKylin
- Android, WearOS
- iOS
- HarmonyOS
- NodeJS
- WebAssembly
- NVIDIA Jetson Orin NX (Support running on both CPU and GPU)
- NVIDIA Jetson Nano B01 (Support running on both CPU and GPU)
- Raspberry Pi
- RV1126
- LicheePi4A
- VisionFive 2
- 旭日X3派
- 爱芯派
- RK3588
- etc

with the following APIs

- C++, C, Python, Go,
  ```
  C#
  ```
- Java, Kotlin, JavaScript
- Swift, Rust
- Dart, Object Pascal... |Description|Huggingface space|ModelScope space|
|--|--|--|
|Voice activity detection with silero-vad|Click me|地址|
|Real-time speech recognition (Chinese + English) with Zipformer|Click me|地址|
|Real-time speech recognition (Chinese + English) with Paraformer|Click me|地址|
|Real-time speech recognition (Chinese + English + Cantonese) with Paraformer-large|Click me|地址|
|Real-time speech recognition (English)|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer CTC|Click me|地址|
|VAD + speech recognition (Chinese + English + Korean + Japanese + Cantonese) with SenseVoice|Click me|地址|
|VAD + speech recognition (English) with Whisper tiny.en|Click me|地址|
|VAD + speech recognition (English) with Moonshine tiny|Click me|地址|
|VAD + speech recognition (English) with Zipformer trained with GigaSpeech|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer trained with WenetSpeech|Click me|地址|
|VAD + speech recognition (Japanese) with Zipformer trained with ReazonSpeech|Click me|地址|
|VAD + speech recognition (Thai) with Zipformer trained with GigaSpeech2|Click me|地址|
|VAD + speech recognition (Chinese 多种方言) with a TeleSpeech-ASR CTC model|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-large|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-small|Click me|地址|
|VAD + speech recognition (多语种及多种中文方言) with Dolphin-base|Click me|地址|
|Speech synthesis (English)|Click me|地址|
|Speech synthesis (German)|Click me|地址|
|Speaker diarization|Click me|地址|... ### Links for pre-trained models

|Description|URL|
|--|--|
|Speech recognition (speech to text, ASR)|Address|
|Text-to-speech (TTS)|Address|
|VAD|Address|
|Keyword spotting|Address|
|Audio tagging|Address|
|Speaker identification (Speaker ID)|Address|
|Spoken language identification (Language ID)|See multi-lingual Whisper ASR models from Speech recognition|
|Punctuation|Address|
|Speaker segmentation|Address|
|Speech enhancement|Address|
|Source separation|Address|

#### Some pre-trained ASR models (Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-ctc/index.html

for more models. The following table lists only **SOME** of them.

|Name|Supported Languages|Description|
|--|--|--|
|sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-small-bilingual-zh-en-2023-02-16|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-zh-14M-2023-02-23|Chinese|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-en-20M-2023-02-17|English|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-korean-2024-06-16|Korean|See also|
|sherpa-onnx-streaming-zipformer-fr-2023-04-14|French|See also|... #### Some pre-trained ASR models (Non-Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-ctc/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/telespeech/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/index.html

for more models. The following table lists only **SOME** of them.... ## About

Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

k2-fsa.github.io/sherpa/onnx/index.html

### Topics

android windows macos linux lazarus raspberry-pi ios text-to-speech csharp cpp dotnet speech-to-text aarch64 mfc risc-v object-pascal asr arm32 onnx vits

### Resources

Readme

### License

Apache-2.0 license

### Uh oh!

There was an error while loading. Please reload this page.

Activity

Custom properties

### Stars

**6.9k** stars

### Watchers

**81** watching

### Forks

**806** forks

## Releases 140

v1.12.7 Latest
Jul 27, 2025

+ 139 releases

## Used by 113
-
- - - - - - - + 105

## Contributors 140
-
- - - - - - - - - - - - - + 126 contributors

## Languages

- C++ 38.3%
- Python 16.2%
- Shell 7.6%
- Kotlin 5.3%
- JavaScript 5.0%
- Dart 4.8%
- Other 22.8%

### 5. GitHub - k2-fsa/sherpa-onnx: Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

**URL:** http://github.com/k2-fsa/sherpa-onnx

# k2-fsa/sherpa-onnx

### Supported functions

|Speech recognition|Speech synthesis|Source separation|
|--|--|--|
|✔️|✔️|✔️|
|Speaker identification|Speaker diarization|Speaker verification|
|--|--|--|
|✔️|✔️|✔️|
|Spoken Language identification|Audio tagging|Voice activity detection|
|--|--|--|
|✔️|✔️|✔️|
|Keyword spotting|Add punctuation|Speech enhancement|
|--|--|--|
|✔️|✔️|✔️|

### Supported platforms

|Architecture|Android|iOS|Windows|macOS|linux|HarmonyOS|
|--|--|--|--|--|--|--|
|x64|✔️| |✔️|✔️|✔️|✔️|
|x86|✔️| |✔️| | | |
|arm64|✔️|✔️|✔️|✔️|✔️|✔️|
|arm32|✔️| | | |✔️|✔️|
|riscv64| | | | |✔️| |

### Supported programming languages

|1. C++|2. C|3. Python|4. JavaScript|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|5. Java|6. C#|7. Kotlin|8. Swift|
|--|--|--|--|
|✔️|✔️|✔️|✔️|
|9. Go|10. Dart|11. Rust|12. Pascal|
|--|--|--|--|
|✔️|✔️|✔️|✔️|

For Rust support, please see sherpa-rs

It also supports WebAssembly.... ## Introduction

This repository supports running the following functions **locally**

- Speech-to-text (i.e., ASR); both streaming and non-streaming are supported
- Text-to-speech (i.e., TTS)
- Speaker diarization
- Speaker identification
- Speaker verification
- Spoken language identification
- Audio tagging
- VAD (e.g., silero-vad)
- Speech enhancement (e.g., gtcrn)
- Keyword spotting
- Source separation (e.g., spleeter, UVR)

on the following platforms and operating systems:

- x86,
  ```
  x86_64
  ```
  , 32-bit ARM, 64-bit ARM (arm64, aarch64), RISC-V (riscv64), **RK NPU**
- Linux, macOS, Windows, openKylin
- Android, WearOS
- iOS
- HarmonyOS
- NodeJS
- WebAssembly
- NVIDIA Jetson Orin NX (Support running on both CPU and GPU)
- NVIDIA Jetson Nano B01 (Support running on both CPU and GPU)
- Raspberry Pi
- RV1126
- LicheePi4A
- VisionFive 2
- 旭日X3派
- 爱芯派
- RK3588
- etc

with the following APIs

- C++, C, Python, Go,
  ```
  C#
  ```
- Java, Kotlin, JavaScript
- Swift, Rust
- Dart, Object Pascal... |Description|Huggingface space|ModelScope space|
|--|--|--|
|Voice activity detection with silero-vad|Click me|地址|
|Real-time speech recognition (Chinese + English) with Zipformer|Click me|地址|
|Real-time speech recognition (Chinese + English) with Paraformer|Click me|地址|
|Real-time speech recognition (Chinese + English + Cantonese) with Paraformer-large|Click me|地址|
|Real-time speech recognition (English)|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer CTC|Click me|地址|
|VAD + speech recognition (Chinese + English + Korean + Japanese + Cantonese) with SenseVoice|Click me|地址|
|VAD + speech recognition (English) with Whisper tiny.en|Click me|地址|
|VAD + speech recognition (English) with Moonshine tiny|Click me|地址|
|VAD + speech recognition (English) with Zipformer trained with GigaSpeech|Click me|地址|
|VAD + speech recognition (Chinese) with Zipformer trained with WenetSpeech|Click me|地址|
|VAD + speech recognition (Japanese) with Zipformer trained with ReazonSpeech|Click me|地址|
|VAD + speech recognition (Thai) with Zipformer trained with GigaSpeech2|Click me|地址|
|VAD + speech recognition (Chinese 多种方言) with a TeleSpeech-ASR CTC model|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-large|Click me|地址|
|VAD + speech recognition (English + Chinese, 及多种中文方言) with Paraformer-small|Click me|地址|
|VAD + speech recognition (多语种及多种中文方言) with Dolphin-base|Click me|地址|
|Speech synthesis (English)|Click me|地址|
|Speech synthesis (German)|Click me|地址|
|Speaker diarization|Click me|地址|... ### Links for pre-trained models

|Description|URL|
|--|--|
|Speech recognition (speech to text, ASR)|Address|
|Text-to-speech (TTS)|Address|
|VAD|Address|
|Keyword spotting|Address|
|Audio tagging|Address|
|Speaker identification (Speaker ID)|Address|
|Spoken language identification (Language ID)|See multi-lingual Whisper ASR models from Speech recognition|
|Punctuation|Address|
|Speaker segmentation|Address|
|Speech enhancement|Address|
|Source separation|Address|

#### Some pre-trained ASR models (Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-ctc/index.html

for more models. The following table lists only **SOME** of them.

|Name|Supported Languages|Description|
|--|--|--|
|sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-small-bilingual-zh-en-2023-02-16|Chinese, English|See also|
|sherpa-onnx-streaming-zipformer-zh-14M-2023-02-23|Chinese|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-en-20M-2023-02-17|English|Suitable for Cortex A7 CPU. See also|
|sherpa-onnx-streaming-zipformer-korean-2024-06-16|Korean|See also|
|sherpa-onnx-streaming-zipformer-fr-2023-04-14|French|See also|... #### Some pre-trained ASR models (Non-Streaming)

Please see

- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-paraformer/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-ctc/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/telespeech/index.html
- https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/index.html

for more models. The following table lists only **SOME** of them.... ## About

Speech-to-text, text-to-speech, speaker diarization, speech enhancement, source separation, and VAD using next-gen Kaldi with onnxruntime without Internet connection. Support embedded systems, Android, iOS, HarmonyOS, Raspberry Pi, RISC-V, x86_64 servers, websocket server/client, support 12 programming languages

k2-fsa.github.io/sherpa/onnx/index.html

### Topics

android windows macos linux lazarus raspberry-pi ios text-to-speech csharp cpp dotnet speech-to-text aarch64 mfc risc-v object-pascal asr arm32 onnx vits

### Resources

Readme

### License

Apache-2.0 license

### Uh oh!

There was an error while loading. Please reload this page.

Activity

Custom properties

### Stars

**6.9k** stars

### Watchers

**81** watching

### Forks

**806** forks

## Releases 140

v1.12.7 Latest
Jul 27, 2025

+ 139 releases

## Used by 113
-
- - - - - - - + 105

## Contributors 140
-
- - - - - - - - - - - - - + 126 contributors

## Languages

- C++ 38.3%
- Python 16.2%
- Shell 7.6%
- Kotlin 5.3%
- JavaScript 5.0%
- Dart 4.8%
- Other 22.8%

## Metadata

```json
{
  "planId": "plan_3",
  "executionTime": 44451,
  "replanned": false
}
```

## Reasoning Insights

- Query complexity: low (0.80)
- Using advanced heuristic analysis

**Confidence:** 85.0%

## Planning Log

```
🎯 GOAP Planning & Execution Log
================================
🧠 Strange Loop Reasoning:
  • Query complexity: low (0.80)
  • Using advanced heuristic analysis
  • Confidence: 85.0%

📋 Plan Execution Summary:
  • Steps executed: 4
  • Success: Yes
  • Replanned: No
```
