# Search Results

**Query:** Parakeet TDT CTC decoder post-processing punctuation restoration
**Date:** 2025-11-12T07:36:06.347Z

## Answer

The **Parakeet TDT CTC decoder** (as in models like `parakeet-tdt_ctc-1.1b` and `parakeet-tdt_ctc-0.6b`) is designed to output **transcriptions with punctuation and capitalization directly, without requiring additional post-processing for punctuation restoration**[4][6][8]. This is a key feature of the model and a major advantage over many traditional ASR pipelines.

---

### How Punctuation Restoration Works in Parakeet TDT CTC

- **Integrated Punctuation and Capitalization:**  
  The Parakeet TDT CTC models are trained on data that includes punctuation and capitalization, so the output text from the model already contains these elements as part of the transcription[4][6][8].
- **No Extra Post-Processing Needed:**  
  Unlike older ASR systems that output plain text and require a separate punctuation restoration model (often a transformer or BERT-based sequence tagger), Parakeet TDT CTC models embed punctuation and capitalization prediction into the main decoding process[4][6][8].
- **Decoder Choice:**  
  - By default, the model uses the **TDT (Token-and-Duration Transducer) decoder**, which is optimized for speed and accuracy.
  - You can switch to the **CTC decoder** by specifying `decoding_type='ctc'` in the NeMo framework, but both decoders output text with punctuation and capitalization if the model was trained for it[4][3].
- **Output Example:**  
  When you run:
  ```python
  output = asr_model.transcribe(['audio.wav'])
  print(output[0].text)
  ```
  The printed text will already include punctuation and capitalization, e.g.:
  ```
  "Hello, how are you? I'm fine, thank you."
  ```
  [4][6][8]

---

### Technical Context

- **Hybrid FastConformer-TDT-CTC Architecture:**  
  The model combines FastConformer encoders with TDT and CTC decoders, trained on large datasets with full punctuation and capitalization in the transcripts[4][2][6].
- **Training on Full Sentences:**  
  Recent research and model documentation emphasize training on long utterances with complete sentences, which improves the model's ability to predict punctuation and capitalization in context[2].
- **Benchmarks:**  
  The models achieve state-of-the-art accuracy on benchmarks, with punctuation and capitalization included in the evaluation[4][6].

---

### If You Need Further Punctuation Restoration

- **For Most Use Cases:**  
  You do **not** need to run a separate punctuation restoration model after decoding with Parakeet TDT CTC. The output is ready for use in downstream applications (subtitling, document generation, etc.)[4][6][8].
- **If Customization Is Needed:**  
  If you want to further refine punctuation (e.g., for a specific style or language variant), you could still apply a post-processing model, but this is generally unnecessary unless you have highly specialized requirements[5].

---

### Summary Table

| Model/Decoder         | Punctuation in Output | Post-Processing Needed? |
|-----------------------|----------------------|-------------------------|
| Parakeet TDT CTC      | Yes                  | No                      |
| Parakeet TDT TDT      | Yes                  | No                      |
| Traditional ASR (no PnC) | No               | Yes (external model)    |

---

### References to Key Documentation

- **Official Model Documentation:**  
  "This model provides transcribed speech as a string for a given audio sample... including punctuation and capitalization of English alphabet"[4].
- **Feature List:**  
  "Generate text with proper punctuation and capitalization without additional post-processing steps"[6][8].
- **Usage Instructions:**  
  "Simply upload your audio file... The system will process your audio and generate a transcript... with proper punctuation and capitalization"[8].

---

**In summary:**  
**Parakeet TDT CTC decoder outputs text with punctuation and capitalization natively, so no additional punctuation restoration post-processing is required.** This is a core feature of the model, enabled by its training data and architecture[4][6][8].

## Citations

### 1. TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning ...

**URL:** https://github.com/NVIDIA-NeMo/NeMo/issues/14140

# TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning on Persian Data #14140

New issue

New issue

Open

Open

TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning on Persian Data#14140

Assignees

Labels

community-request

## Description

saeedzou

opened on Jul 6, 2025 · edited by saeedzou

[update 2025-07-07]: I initialized a new run with
```
batch_size=8
```
and
```
accumulate_grad_batches=128
```
as mentioned by @MahmoudAshraf97 [update 2025-07-07]: I stopped the training to pull the new changes and apply the second modification mentioned by @jeremy110

## TODO
- set the ctc-weight to zero to see how the TDT head does by itself.

  To pick up a draggable item, press the space bar.
  While dragging, use the arrow keys to move the item.
  Press space again to drop the item in its new position, or press escape to cancel.

 @nithinraok

## Description of Issue:

I am fine-tuning the parakeet-tdt_ctc-110m ASR model on approximately 1800 hours of Persian speech data, following the speech_to_text_finetune.py recipe. I've also trained a 1024-token BPE tokenizer on the training data and updated the model configuration to use it.

After 9 epochs of training, I've observed that while the CTC head's performance is improving steadily (as indicated by train_ctc_loss, training_batch_wer_ctc, and val_wer_ctc), the TDT head shows little to no improvement, with its WER remaining high (visible when comparing training_batch_wer and val_wer against their CTC counterparts).

Another issue is that the predicted text in the logs are generating the token "⁇", example: [NeMo I 2025-07-07 02:43:21 wer:330] reference:درست مانند منشور که نور را به طیفی از رنگ‌های مختلف تقسیم می‌کند. [NeMo I 2025-07-07 02:43:21 wer:331] predicted: ⁇ راست ماننسور که نور را شریر‌ مختلف، تقس می

Note that this is happening in the predicted sentences not the references, ruling out the possibility of out of vocab tokens in the reference. This happens rarely though. CTC head used to make this token at first but now there is no sign. Maybe there is something wrong with the inference code?!

Below is the relevant portion of my train_config.yaml with paths replaced for privacy:

**Configuration**

```
name: "parakeet_tdt_ctc_110m_v41_punc/2025-07-04_18-02-09"

# Initialize from your local .nemo instead of HF
init_from_nemo_model: "parakeet-tdt_ctc-110m.nemo"
init_from_pretrained_model: null

model:
  sample_rate: 16000

  train_ds:
    manifest_filepath: "ASR/Datasets/Final_Train_Normalized_Punc.json"
    sample_rate: ${model.sample_rate}
    batch_size: 6 # you may increase batch_size if your memory allows
    shuffle: true
    num_workers: 8
    pin_memory: true
    max_duration: 30
    min_duration: null
    channel_selector: "average" # Averages stereo channels to mono
    # tarred datasets
    is_tarred: false
    tarred_audio_filepaths: null
    shuffle_n: 2048
    # bucketing params
    bucketing_strategy: "fully_randomized"
    bucketing_batch_size: null

  validation_ds:
    manifest_filepath: "ASR/Datasets/Final_Valid_Normalized_Punc.json"
    sample_rate: ${model.sample_rate}
    batch_size: 32
    max_duration: 30
    min_duration: null
    channel_selector: "average" # Averages stereo channels to mono
    shuffle: false
    use_start_end_token: false
    num_workers: 8
    pin_memory: true

  char_labels: # use for char based models
  update_labels: false
  labels: null # example list config: \[' ', 'a', 'b', 'c'\]

  # Swap in your Persian BPE tokenizer under model
  tokenizer:
    update_tokenizer: true
    dir: "ASR/Tokenizers/V41_Nemo_Tokenizer_1024/tokenizer_spe_bpe_v1024_max_4"
    type: bpe
  # SpecAugment (unchanged)
  spec_augment:
    _target_: nemo.collections.asr.modules.SpectrogramAugmentation
    freq_masks: 2
    time_masks: 10
    freq_width: 27
    time_width: 0.05

  # ORIGINAL optimizer & scheduler from speech_to_text_finetune.yaml
  optim:
    name: adamw
    lr: 1e-4
    betas: [0.9, 0.98]
    weight_decay: 1e-3
    sched:
      name: CosineAnnealing
      warmup_steps: 5000
      warmup_ratio: null
      min_lr: 5e-6

trainer:
  devices: -1
  num_nodes: 1
  max_epochs: 50
  max_steps: -1 # computed at runtime if not set
  val_check_interval: 1.0 # Set to 0.25 to check 4 times per epoch, or an int for number of iterations
  accelerator: auto
  strategy:
    _target_: lightning.pytorch.strategies.DDPStrategy
    gradient_as_bucket_view: true
  accumulate_grad_batches: 1
  gradient_clip_val: 0.0
  precision: 32 # Use mixed precision # 16, 32, or bf16
  log_every_n_steps: 10 # Interval of logging.
  enable_progress_bar: True
  num_sanity_val_steps: 0 # number of steps to perform validation steps for sanity check the validation process before starting the training, setting to 0 disables it
  check_val_every_n_epoch: 1 # number of evaluations on validation every n epochs
  sync_batchnorm: true
  enable_checkpointing: False # Provided by exp_manager
  logger: false # Provided by exp_manager
  benchmark: false # needs to be false for models with variable-length speech input as it slows down training

exp_manager:
  exp_dir: "ASR/exp"
  name: ${name}
  create_tensorboard_logger: true
  create_checkpoint_callback: true
  checkpoint_callback_params:
    monitor: "val_wer" # Use epoch as a dummy monitor metric
    mode: "min" # Higher epoch is better
    every_n_epochs: 1 # Save every epoch
    save_top_k: -1 # Save all checkpoints
    save_last: true # Do not only keep a "last" file
    save_on_train_epoch_end: True # Save after validation
    always_save_nemo: true # Save .nemo file each time
    filename: "model-{epoch:02d}-{val_wer:.2f}" # Unique filename with epoch & metric

  resume_if_exists: true
  resume_ignore_no_checkpoint: false

  create_wandb_logger: false
  wandb_logger_kwargs:
    name: null
    project: null
```

** Attached Plots**
(Plots will be attached separately, showing trends for epochs, train_loss, train_rnnt_loss, train_ctc_loss, training_batch_wer, training_batch_wer_ctc, val_wer, and val_wer_ctc.)

## Possible Reasons for TDT Stagnation:
1. Tokenizer Mismatch/Optimization for TDT: While a new tokenizer is used, the TDT head's prediction network might not be effectively adapting to the new token set or the specific characteristics of Persian language (e.g., common sequences, subword units) in the same way the CTC head does.
2. Learning Rate and Schedule: The current learning rate (1e-4) and CosineAnnealing schedule with 5000 warmup steps might not be optimal for the TDT head to sufficiently adapt, especially given the new language and tokenizer. TDT often requires more careful tuning of these parameters.

## System Configuration
- Operating System: Ubuntu 22.04.1 LTS
- Kernel: Linux eri4090 6.8.0-60-generic x86_64
- GPU: NVIDIA GeForce RTX 4090
- NVIDIA Driver Version: 560.35.03
- CUDA Version: 12.6
- GPU Memory: 24 GB
- NeMo Toolkit Version: 2.4.0rc0
- Torch Version: 2.6.0+cu126

I appreciate any insights or suggestions to resolve this issue.

### 2. How To Use - 3 Simple Steps

**URL:** https://parakeettdt.org

The intuitive Parakeet TDT platform makes converting speech to text remarkably simple. Follow these steps to transcribe audio with industry-leading speed and accuracy.

Upload audio files in common formats. The system accepts everything from short clips to hour-long recordings with equal efficiency.

Select transcription parameters including timestamp precision, punctuation preferences, and output format options (available in more advanced integrations).

Process audio at unprecedented speed and download perfectly formatted text transcripts ready for immediate use from the demo or your integrated solution.

Discover the powerful speech recognition technology that transcribes audio with remarkable speed and precision while requiring minimal computational resources.

Transcribe 60 minutes of audio in just 1 second with the efficient 0.6B parameter model architecture.

Achieve high accuracy (e.g., ~6% WER on benchmarks, claimed 98% on specific long audio tests) on long audio files with state-of-the-art recognition capabilities.

Generate text with proper punctuation and capitalization without additional post-processing steps.

Receive accurate word-level timestamps for perfect synchronization between audio and transcribed text.

Deploy efficiently with only 0.6B parameters, requiring significantly less computational resources than some comparable models.

Benefit from a top-ranked speech recognition model on industry standard OpenASR benchmarks for the English language.

See how Parakeet TDT revolutionary speech recognition capabilities are transforming transcription workflows and enabling new possibilities across industries.... Podcast Producer

"Parakeet TDT has revolutionized our audio transcription process. The ability to process 60-minute episodes in just seconds allows us to create accurate transcripts immediately. The recognition quality is incredible — even with multiple speakers and background noise. The automatic punctuation and capitalization has eliminated hours of manual editing work."

Conference Organizer

"As someone who works with hours of recorded presentations, Parakeet TDT 0.6B approach to speech recognition is groundbreaking. The precise timestamps and exceptional accuracy are unlike anything available before. I can transcribe entire conferences with consistent quality, which has opened up entirely new accessibility options."

Content Creator

"Parakeet TDT 0.6B recognition feature has transformed my workflow. I can upload lengthy interviews and receive perfectly formatted transcripts almost instantly. The lightweight model runs efficiently even on standard hardware. Plus, the high accuracy rate means minimal editing is needed before publication."

E-Learning Developer

"Parakeet TDT transcription consistency is unmatched in the industry. The output quality across different speakers shows incredible accuracy and detail. The ability to process long educational content has streamlined our course development process significantly. It has become an essential tool in our educational content arsenal."

Research Director

"Parakeet TDT speed and quality are remarkable. I can quickly transcribe multiple interviews for research projects, maintaining consistent accuracy throughout. The natural handling of technical terminology makes our work significantly easier. It has completely changed how we approach qualitative research data processing."... Media Accessibility Specialist

"Parakeet TDT speech recognition technology has revolutionized our subtitle creation process. The ability to generate accurate transcripts with precise timestamps gives us unprecedented efficiency. The instant processing and exceptional accuracy have become integral to our media accessibility workflow."

Find answers to common questions about Parakeet TDT speech recognition technology. Need more help? Contact our support team at [email protected].

Simply upload your audio file through the interface to convert it to accurately transcribed text. The system will process your audio and generate a transcript with remarkable speed. You can adjust parameters like timestamp precision, punctuation preferences, and output format (in advanced integrations). The ultra-fast processing allows you to receive results almost instantly.

Parakeet TDT 0.6B processes audio at unprecedented speeds - approximately 60 minutes of audio in just 1 second on appropriate hardware. Even lengthy recordings are transcribed almost instantly. Once transcription is complete, you can view, download, or share your high-quality text output with precise timestamps.

We take your privacy seriously. For the embedded Hugging Face demo, please refer to their privacy policy. When using the model via NVIDIA NeMo or other self-hosted solutions, data handling is under your control. For any service offered directly on this site (if applicable in the future), all audio inputs would be encrypted during transmission and processing. We would not store your audio files or generated transcripts beyond the current session unless you explicitly save them. Our systems would comply with industry-standard security protocols to ensure your data remains protected.... Parakeet TDT supports common audio formats including MP3, WAV, M4A, FLAC, and OGG. The system can handle various audio qualities, though clearer recordings with minimal background noise will yield the most accurate results. The model is trained to handle natural speech patterns across different speakers.

Yes, Parakeet TDT models are typically released under permissive licenses like CC-BY-4.0, which allows for commercial use of the model's output. You retain full ownership of the generated content and can use it in products, services, documentation, or any other commercial applications without additional licensing fees from the model itself.

Parakeet TDT 0.6B achieves excellent accuracy on standard benchmarks (e.g., a Word Error Rate of ~6.05% on the Hugging Face Open ASR Leaderboard). Performance may vary slightly based on audio quality, speaker clarity, and background noise. The model excels at recognizing natural conversational speech and automatically adds appropriate punctuation and capitalization.

The Parakeet-TDT-0.6B-v2 model features 600 million parameters. It combines a FastConformer encoder with a Token-and-Duration Transducer (TDT) decoder. This architecture is optimized for NVIDIA GPUs (like A100, H100, T4, V100) and can transcribe an hour of audio in approximately one second, achieving a Real-Time Factor (RTFx) of around 3386 with a batch size of 128.... It's trained on diverse, large-scale datasets such as the Granary dataset (approx. 120,000 hours of English audio), ensuring robustness across various accents and noise conditions. The model supports punctuation, capitalization, and detailed word-level timestamping.

While optimized for GPUs, it can be loaded on systems with as little as 2GB of RAM for broader deployment, though performance will vary.

### 3. Canary-1B-v2 & Parakeet-TDT-0.6B-v3 - arXiv

**URL:** https://arxiv.org/html/2509.14128v2

## 1 Introduction

Modern speech processing systems for Automatic Speech Recognition (ASR) and Speech Translation (ST) are predominantly built on the encoder-decoder paradigm. Architectures such as the TransformerVaswani et al. (2023) have proven highly effective, with models like WhisperRadford et al. (2023) and SeamlessM4TBarrault & Others (2023) demonstrating that training on massive, weakly supervised multilingual datasets can yield state-of-the-art results. Despite their strong performance, these models often require large resources and run slowly, creating a trade-off between accuracy, size, and speed. Specialized architectures like Conformer and its efficient variant, FastConformer, have been developed to better capture speech-specific features with a smaller computation footprint.

This work introduces Canary-1B-v2, a multilingual, multi-task model designed to deliver robust ASR and AST performance with high efficiency. In addition, we release Parakeet-TDT-0.6B-v3, a smaller yet more accurate ASR model supporting the same 25 languages.

The key contributions of this paper are:

- •

  A Multi-Stage Training and Fine-Tuning Strategy: A two-stage pre-training regimen followed by a high-quality fine-tuning stage with dynamic weight scheduling to address significant data imbalances in large-scale multilingual corpora.
- •

  Comprehensive Data Curation: Training on a 1.7 million-hour dataset spanning 25 languages, combining pseudo-labeled and human-annotated data, and incorporating non-speech audio to improve robustness.
- •

  Efficient Architecture and Timestamp Generation: Canary-1B-v2 leverages a FastConformer encoder and a unified BPE tokenizer for all languages, delivering high throughput. It integrates the NeMo Forced Aligner (NFA) with an auxiliary CTC model for accurate segment-level timestamps in both ASR and AST tasks.
- •

  State-of-the-Art Performance: Canary-1B-v2 achieves competitive or superior results to much larger models on ASR and AST benchmarks, including the Hugging Face Open ASR Leaderboard, while providing significantly faster inference.

This report details the models’ architectures, the data curation and balancing strategies, the multi-stage training process, and extensive evaluations against leading speech processing systems.... Based on these analyses, Canary-1B-v2 employs a unified BPE tokenizerSennrich et al. (2016) to handle input across all 25 languages, providing robust multilingual processing capabilities while maintaining computational efficiency.... *Parakeet-TDT-0.6B-v3* simialr to Canary-1b-v2 used FastConformer encoder with 24 layers and a TDT decoderXu et al. (2023). This model was first initialized from a multilingual CTC checkpoint pre-trained on the Granary ASR subset and trained for 150,000 steps on 128 A100 GPUs with temperature-based sampling (\alpha=0.5 and \beta=0.5) to balance corpora and languages, as described in Section 3.3.1.

|Regime|Model|HF Leaderboard (En)|FLEURS (25)|MLS (5)|CoVoST2 (13)|
|--|--|--|--|--|--|
|Two-stage|X\toEn|7.54%|-|-|-|
| |X\toEn + ASR + En\toX|8.36%|11.03%|7.21%|8.31%|
|Single-stage|X\toEn + ASR + En\toX|7.73%|10.66%|7.35%|8.11%|
(a) ASR results (WER). Lower is better.... ### 4.3 Fine-tuning

As discussed in Section 3.3, a recurring challenge in our training pipeline is single-corpus or domain domination, which affects more than half of the supported languages. This has already impacted our results: for instance, we consistently observed unusually low (below SOTA) performance in the X→En direction, where VoxPopuli dominance is particularly evident.

To mitigate this and other imbalances in the pre-training data, we extended the pre-trained checkpoint with a a third high-quality fine-tuning stage. Importantly, this fine-tuning does not introduce a new task or domain but instead focuses the model on a high-quality subset of the training data. This subset includes NeMo ASR Set 3.0 and the YouTube portion of the Granary dataset for ASR and X→En, as well as our supplementary dataset for En→X. Additionally, we filtered the translation training data by QE scores^6^^6^6QE scores computed with Unbabel/wmt22-comet-daGowda et al. (2023), keeping only samples with QE > 0.85. Thus, beyond mitigating corpus dominance, this stage also addresses quality imbalances within individual corpora.

To further balance tasks and languages, we constructed the training data as follows: for each language pair, we selected 200 hours of data from the high-quality subset. If fewer than 200 hours were available, the remainder was filled with other data (including VoxPopuli when necessary). This construction strategy was based on similar steps followed for
*parakeet-tdt-0.6b-v2*^7^^7^7https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2 , where it had already yielded remarkable results. The complete training set was then divided into four equally weighted groups: ASR (non-English) (4,800 h), X→En (4,800 h), En→X (4,800 h), and English ASR (600 h). During training, sampling was balanced across groups, with each group selected with equal probability. Within each group, language pairs were also equally likely to be sampled, since their data volumes were normalized to 200 hours per pair.... In light of these findings, we release the weight-scheduled fine-tuned model as our primary version and continue to explore data balancing schedules to better understand their benefits and broader applicability.

Fine-tuning stage for *Parakeet-TDT-0.6B-v3* ran for 5,000 steps on 4 A100 GPUs using 7,500 hours of high-quality NeMo ASR Set 3.0 data.... ### 5.1 Timestamps Generation in Attention-Encoder Decoder Models

The extraction of precise timestamps from spoken words is a critical capability of modern Speech Recognition systems, essential for powering downstream applications such as timed subtitling and content retrieval. The methods for extracting timestamps from attention-based speech recognition models are diverse, ranging from post-processing techniques to architectural redesigns. Each approach represents a different trade-off between accuracy, computational overhead, and architectural complexity.

The architecture that we are working with in Canary-1B-v2 is an attention-based encoder-decoder (AED) system. In these models, alignment between audio frames and text tokens is handled during the decoding phase. This is primarily facilitated by the cross-attention mechanism, which produces a weight matrix representing the probabilistic correlation between each output token and every input audio frame. While these weights provide a "soft" alignment, they are prone to non-monotonicity, where the attention path can jump back and forth in time, leading to a certain degree of inherent unreliability or "fuzziness."

#### 5.1.1 Post-Processing with Dynamic Time Warping (DTW)

To convert this soft, often non-monotonic alignment into precise, word-level boundaries, a robust post-processing algorithm like Dynamic Time Warping (DTW)Giorgino (2009) is applied to the cross-attention matrix. DTW finds the optimal, non-linear alignment path between two sequences by computing a cumulative cost or score matrix. In this context, it aligns the sequence of output tokens with the sequence of input audio frames by finding the path of highest cumulative attention scores. This process effectively "hardens" the fuzzy attention into a clear, monotonic sequence of word boundaries, providing the start and end times for each word. A prominent example of this method is its use in third-party implementationsLouradour (2023) for WhisperRadford et al. (2023) model, which natively provides only segment-level timestamps.... Additionally, the CTC model (Parakeet CTC) incorporated into the NFA pipeline is trained solely for ASR tasks and lacks AST knowledge.

To evaluate timestamp generation capabilities for AST tasks, we conducted experiments on evaluation benchmarks and performed manual evaluation of the generated timestamps. The NFA pipeline integrated into Canary-1B-v2’s inference mechanism successfully produced high-quality segment-level timestamps, as shown in Figure 8b. We therefore recommend using segment-level timestamps with translation outputs, as word-level timestamps can be inaccurate due to the non-monotonic nature of speech translation. An example of segment-level timestamps with the respective prompts is illustrated in Figure 9.

Overall, we observed that the current pipeline with NFA yields consistently reliable timestamping performance. We attribute this in part to the fact that our evaluation scope primarily covers European languages, where translations to and from English tend to preserve sentence structure without drastic reordering. By contrast, languages with more divergent syntactic structures such as Mandarin may introduce substantial shifts in word and phrase order. As such, while the results within our explored language set are promising, the applicability of the NFA pipeline to typologically distant languages remains uncertain and warrants more thorough testing and investigation.

Looking ahead, future work will focus on more comprehensive and quantitative AST timestamp evaluation and on exploring how log-probabilities of an ASR model are effectively leveraged for aligning translation transcript tokens.

### 4. TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning ...

**URL:** https://github.com/NVIDIA-NeMo/NeMo/issues/14140

# TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning on Persian Data #14140

New issue

New issue

Open

Open

TDT Head Stagnation in parakeet-tdt_ctc-110m Fine-tuning on Persian Data#14140

Assignees

Labels

community-request

## Description

saeedzou

opened on Jul 6, 2025 · edited by saeedzou

[update 2025-07-07]: I initialized a new run with
```
batch_size=8
```
and
```
accumulate_grad_batches=128
```
as mentioned by @MahmoudAshraf97 [update 2025-07-07]: I stopped the training to pull the new changes and apply the second modification mentioned by @jeremy110

## TODO
- set the ctc-weight to zero to see how the TDT head does by itself.

  To pick up a draggable item, press the space bar.
  While dragging, use the arrow keys to move the item.
  Press space again to drop the item in its new position, or press escape to cancel.

 @nithinraok

## Description of Issue:

I am fine-tuning the parakeet-tdt_ctc-110m ASR model on approximately 1800 hours of Persian speech data, following the speech_to_text_finetune.py recipe. I've also trained a 1024-token BPE tokenizer on the training data and updated the model configuration to use it.

After 9 epochs of training, I've observed that while the CTC head's performance is improving steadily (as indicated by train_ctc_loss, training_batch_wer_ctc, and val_wer_ctc), the TDT head shows little to no improvement, with its WER remaining high (visible when comparing training_batch_wer and val_wer against their CTC counterparts).

Another issue is that the predicted text in the logs are generating the token "⁇", example: [NeMo I 2025-07-07 02:43:21 wer:330] reference:درست مانند منشور که نور را به طیفی از رنگ‌های مختلف تقسیم می‌کند. [NeMo I 2025-07-07 02:43:21 wer:331] predicted: ⁇ راست ماننسور که نور را شریر‌ مختلف، تقس می

Note that this is happening in the predicted sentences not the references, ruling out the possibility of out of vocab tokens in the reference. This happens rarely though. CTC head used to make this token at first but now there is no sign. Maybe there is something wrong with the inference code?!

Below is the relevant portion of my train_config.yaml with paths replaced for privacy:

**Configuration**

```
name: "parakeet_tdt_ctc_110m_v41_punc/2025-07-04_18-02-09"

# Initialize from your local .nemo instead of HF
init_from_nemo_model: "parakeet-tdt_ctc-110m.nemo"
init_from_pretrained_model: null

model:
  sample_rate: 16000

  train_ds:
    manifest_filepath: "ASR/Datasets/Final_Train_Normalized_Punc.json"
    sample_rate: ${model.sample_rate}
    batch_size: 6 # you may increase batch_size if your memory allows
    shuffle: true
    num_workers: 8
    pin_memory: true
    max_duration: 30
    min_duration: null
    channel_selector: "average" # Averages stereo channels to mono
    # tarred datasets
    is_tarred: false
    tarred_audio_filepaths: null
    shuffle_n: 2048
    # bucketing params
    bucketing_strategy: "fully_randomized"
    bucketing_batch_size: null

  validation_ds:
    manifest_filepath: "ASR/Datasets/Final_Valid_Normalized_Punc.json"
    sample_rate: ${model.sample_rate}
    batch_size: 32
    max_duration: 30
    min_duration: null
    channel_selector: "average" # Averages stereo channels to mono
    shuffle: false
    use_start_end_token: false
    num_workers: 8
    pin_memory: true

  char_labels: # use for char based models
  update_labels: false
  labels: null # example list config: \[' ', 'a', 'b', 'c'\]

  # Swap in your Persian BPE tokenizer under model
  tokenizer:
    update_tokenizer: true
    dir: "ASR/Tokenizers/V41_Nemo_Tokenizer_1024/tokenizer_spe_bpe_v1024_max_4"
    type: bpe
  # SpecAugment (unchanged)
  spec_augment:
    _target_: nemo.collections.asr.modules.SpectrogramAugmentation
    freq_masks: 2
    time_masks: 10
    freq_width: 27
    time_width: 0.05

  # ORIGINAL optimizer & scheduler from speech_to_text_finetune.yaml
  optim:
    name: adamw
    lr: 1e-4
    betas: [0.9, 0.98]
    weight_decay: 1e-3
    sched:
      name: CosineAnnealing
      warmup_steps: 5000
      warmup_ratio: null
      min_lr: 5e-6

trainer:
  devices: -1
  num_nodes: 1
  max_epochs: 50
  max_steps: -1 # computed at runtime if not set
  val_check_interval: 1.0 # Set to 0.25 to check 4 times per epoch, or an int for number of iterations
  accelerator: auto
  strategy:
    _target_: lightning.pytorch.strategies.DDPStrategy
    gradient_as_bucket_view: true
  accumulate_grad_batches: 1
  gradient_clip_val: 0.0
  precision: 32 # Use mixed precision # 16, 32, or bf16
  log_every_n_steps: 10 # Interval of logging.
  enable_progress_bar: True
  num_sanity_val_steps: 0 # number of steps to perform validation steps for sanity check the validation process before starting the training, setting to 0 disables it
  check_val_every_n_epoch: 1 # number of evaluations on validation every n epochs
  sync_batchnorm: true
  enable_checkpointing: False # Provided by exp_manager
  logger: false # Provided by exp_manager
  benchmark: false # needs to be false for models with variable-length speech input as it slows down training

exp_manager:
  exp_dir: "ASR/exp"
  name: ${name}
  create_tensorboard_logger: true
  create_checkpoint_callback: true
  checkpoint_callback_params:
    monitor: "val_wer" # Use epoch as a dummy monitor metric
    mode: "min" # Higher epoch is better
    every_n_epochs: 1 # Save every epoch
    save_top_k: -1 # Save all checkpoints
    save_last: true # Do not only keep a "last" file
    save_on_train_epoch_end: True # Save after validation
    always_save_nemo: true # Save .nemo file each time
    filename: "model-{epoch:02d}-{val_wer:.2f}" # Unique filename with epoch & metric

  resume_if_exists: true
  resume_ignore_no_checkpoint: false

  create_wandb_logger: false
  wandb_logger_kwargs:
    name: null
    project: null
```

** Attached Plots**
(Plots will be attached separately, showing trends for epochs, train_loss, train_rnnt_loss, train_ctc_loss, training_batch_wer, training_batch_wer_ctc, val_wer, and val_wer_ctc.)

## Possible Reasons for TDT Stagnation:
1. Tokenizer Mismatch/Optimization for TDT: While a new tokenizer is used, the TDT head's prediction network might not be effectively adapting to the new token set or the specific characteristics of Persian language (e.g., common sequences, subword units) in the same way the CTC head does.
2. Learning Rate and Schedule: The current learning rate (1e-4) and CosineAnnealing schedule with 5000 warmup steps might not be optimal for the TDT head to sufficiently adapt, especially given the new language and tokenizer. TDT often requires more careful tuning of these parameters.

## System Configuration
- Operating System: Ubuntu 22.04.1 LTS
- Kernel: Linux eri4090 6.8.0-60-generic x86_64
- GPU: NVIDIA GeForce RTX 4090
- NVIDIA Driver Version: 560.35.03
- CUDA Version: 12.6
- GPU Memory: 24 GB
- NeMo Toolkit Version: 2.4.0rc0
- Torch Version: 2.6.0+cu126

I appreciate any insights or suggestions to resolve this issue.

### 5. How To Use - 3 Simple Steps

**URL:** https://parakeettdt.org

The intuitive Parakeet TDT platform makes converting speech to text remarkably simple. Follow these steps to transcribe audio with industry-leading speed and accuracy.

Upload audio files in common formats. The system accepts everything from short clips to hour-long recordings with equal efficiency.

Select transcription parameters including timestamp precision, punctuation preferences, and output format options (available in more advanced integrations).

Process audio at unprecedented speed and download perfectly formatted text transcripts ready for immediate use from the demo or your integrated solution.

Discover the powerful speech recognition technology that transcribes audio with remarkable speed and precision while requiring minimal computational resources.

Transcribe 60 minutes of audio in just 1 second with the efficient 0.6B parameter model architecture.

Achieve high accuracy (e.g., ~6% WER on benchmarks, claimed 98% on specific long audio tests) on long audio files with state-of-the-art recognition capabilities.

Generate text with proper punctuation and capitalization without additional post-processing steps.

Receive accurate word-level timestamps for perfect synchronization between audio and transcribed text.

Deploy efficiently with only 0.6B parameters, requiring significantly less computational resources than some comparable models.

Benefit from a top-ranked speech recognition model on industry standard OpenASR benchmarks for the English language.

See how Parakeet TDT revolutionary speech recognition capabilities are transforming transcription workflows and enabling new possibilities across industries.... Podcast Producer

"Parakeet TDT has revolutionized our audio transcription process. The ability to process 60-minute episodes in just seconds allows us to create accurate transcripts immediately. The recognition quality is incredible — even with multiple speakers and background noise. The automatic punctuation and capitalization has eliminated hours of manual editing work."

Conference Organizer

"As someone who works with hours of recorded presentations, Parakeet TDT 0.6B approach to speech recognition is groundbreaking. The precise timestamps and exceptional accuracy are unlike anything available before. I can transcribe entire conferences with consistent quality, which has opened up entirely new accessibility options."

Content Creator

"Parakeet TDT 0.6B recognition feature has transformed my workflow. I can upload lengthy interviews and receive perfectly formatted transcripts almost instantly. The lightweight model runs efficiently even on standard hardware. Plus, the high accuracy rate means minimal editing is needed before publication."

E-Learning Developer

"Parakeet TDT transcription consistency is unmatched in the industry. The output quality across different speakers shows incredible accuracy and detail. The ability to process long educational content has streamlined our course development process significantly. It has become an essential tool in our educational content arsenal."

Research Director

"Parakeet TDT speed and quality are remarkable. I can quickly transcribe multiple interviews for research projects, maintaining consistent accuracy throughout. The natural handling of technical terminology makes our work significantly easier. It has completely changed how we approach qualitative research data processing."... Media Accessibility Specialist

"Parakeet TDT speech recognition technology has revolutionized our subtitle creation process. The ability to generate accurate transcripts with precise timestamps gives us unprecedented efficiency. The instant processing and exceptional accuracy have become integral to our media accessibility workflow."

Find answers to common questions about Parakeet TDT speech recognition technology. Need more help? Contact our support team at [email protected].

Simply upload your audio file through the interface to convert it to accurately transcribed text. The system will process your audio and generate a transcript with remarkable speed. You can adjust parameters like timestamp precision, punctuation preferences, and output format (in advanced integrations). The ultra-fast processing allows you to receive results almost instantly.

Parakeet TDT 0.6B processes audio at unprecedented speeds - approximately 60 minutes of audio in just 1 second on appropriate hardware. Even lengthy recordings are transcribed almost instantly. Once transcription is complete, you can view, download, or share your high-quality text output with precise timestamps.

We take your privacy seriously. For the embedded Hugging Face demo, please refer to their privacy policy. When using the model via NVIDIA NeMo or other self-hosted solutions, data handling is under your control. For any service offered directly on this site (if applicable in the future), all audio inputs would be encrypted during transmission and processing. We would not store your audio files or generated transcripts beyond the current session unless you explicitly save them. Our systems would comply with industry-standard security protocols to ensure your data remains protected.... Parakeet TDT supports common audio formats including MP3, WAV, M4A, FLAC, and OGG. The system can handle various audio qualities, though clearer recordings with minimal background noise will yield the most accurate results. The model is trained to handle natural speech patterns across different speakers.

Yes, Parakeet TDT models are typically released under permissive licenses like CC-BY-4.0, which allows for commercial use of the model's output. You retain full ownership of the generated content and can use it in products, services, documentation, or any other commercial applications without additional licensing fees from the model itself.

Parakeet TDT 0.6B achieves excellent accuracy on standard benchmarks (e.g., a Word Error Rate of ~6.05% on the Hugging Face Open ASR Leaderboard). Performance may vary slightly based on audio quality, speaker clarity, and background noise. The model excels at recognizing natural conversational speech and automatically adds appropriate punctuation and capitalization.

The Parakeet-TDT-0.6B-v2 model features 600 million parameters. It combines a FastConformer encoder with a Token-and-Duration Transducer (TDT) decoder. This architecture is optimized for NVIDIA GPUs (like A100, H100, T4, V100) and can transcribe an hour of audio in approximately one second, achieving a Real-Time Factor (RTFx) of around 3386 with a batch size of 128.... It's trained on diverse, large-scale datasets such as the Granary dataset (approx. 120,000 hours of English audio), ensuring robustness across various accents and noise conditions. The model supports punctuation, capitalization, and detailed word-level timestamping.

While optimized for GPUs, it can be loaded on systems with as little as 2GB of RAM for broader deployment, though performance will vary.

## Metadata

```json
{
  "planId": "plan_2",
  "executionTime": 36750,
  "replanned": false
}
```

## Reasoning Insights

- Query complexity: low (0.70)
- Using advanced heuristic analysis

**Confidence:** 85.0%

## Planning Log

```
🎯 GOAP Planning & Execution Log
================================
🧠 Strange Loop Reasoning:
  • Query complexity: low (0.70)
  • Using advanced heuristic analysis
  • Confidence: 85.0%

📋 Plan Execution Summary:
  • Steps executed: 4
  • Success: Yes
  • Replanned: No
```
