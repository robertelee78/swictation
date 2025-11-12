# Search Results

**Query:** NeMo ASR streaming punctuation capitalization models inference
**Date:** 2025-11-12T07:37:48.366Z

## Answer

NVIDIA NeMo provides **punctuation and capitalization (PnC) models** that can be used to restore punctuation and proper casing to the output of ASR (Automatic Speech Recognition) systems, including in streaming inference scenarios. Below is a detailed overview of how these models work, their types, inference process, and considerations for streaming ASR pipelines.

---

### **Types of NeMo Punctuation and Capitalization Models**

NeMo offers two main types of PnC models[2][4]:

- **Lexical-only models:**  
  These models use only the text (lexical features) as input. They are typically based on pre-trained language models such as BERT or T5, with two token-level classification heads: one for punctuation and one for capitalization.

- **Lexical and audio models:**  
  These models (still under development in NeMo 2.0) use both the ASR transcript and audio features, aiming for improved accuracy by leveraging prosodic cues from speech.

---

### **Model Architecture and Data Format**

- **Joint Training:**  
  The standard NeMo PnC model is a joint model with two token-level classifiers on top of a pre-trained language model (e.g., BERT, T5)[4][8]. One classifier predicts punctuation (e.g., period, comma, question mark, or none), and the other predicts capitalization (e.g., upper-case or not).

- **Data Format:**  
  - **text.txt:** Each line contains a sequence of words (ASR output).
  - **labels.txt:** Each line contains corresponding labels for each word, where each label is a two-character code:  
    - First character: punctuation (`O` for none, `,` for comma, `.` for period, `?` for question mark, etc.)
    - Second character: capitalization (`U` for upper-case, `O` for no capitalization)[4].

---

### **Inference with NeMo PnC Models**

#### **Batch/Offline Inference**

- **Script-based Inference:**  
  You can run inference using the provided script:
  ```bash
  python punctuate_capitalize_infer.py \
    --input_manifest <PATH/TO/INPUT/MANIFEST> \
    --output_manifest <PATH/TO/OUTPUT/MANIFEST> \
    --pretrained_name punctuation_en_bert \
    --max_seq_length 64 \
    --margin 16 \
    --step 8
  ```
  - The script processes the ASR output (either from `'pred_text'` or `'text'` fields in the manifest) and writes the punctuated and capitalized text to the output manifest[4].

- **Sliding Window:**  
  For long sequences, the model uses a sliding window approach with overlapping segments to minimize boundary errors. Margins are discarded to avoid artifacts at segment edges[8].

#### **Streaming Inference**

- **Streaming ASR Output:**  
  Most streaming ASR models (e.g., FastConformer, Cache-aware Streaming Conformer) output text without punctuation or capitalization[6]. To restore PnC in real time:
    - **Option 1:** Buffer the ASR output in chunks (e.g., sentences or fixed-length windows), then apply the PnC model to each chunk as it becomes available.
    - **Option 2:** Use a streaming-capable PnC model (if available), which can process partial outputs and update predictions as more context arrives.

- **Latency vs. Accuracy Trade-off:**  
  - Shorter chunks yield lower latency but may reduce PnC accuracy due to limited context.
  - Longer chunks improve accuracy but increase latency, as the model needs more context to make reliable predictions[5][6].

- **Integration Example:**  
  In NeMo, you can integrate PnC restoration into a streaming pipeline by:
    1. Collecting ASR output in a buffer.
    2. Running the PnC model on the buffered text (using the same inference script or via direct API calls).
    3. Emitting the punctuated and capitalized text downstream.

---

### **Model Performance and Limitations**

- **Accuracy:**  
  Jointly trained models (e.g., BERT/T5-based) achieve strong performance on standard benchmarks, but may struggle with:
    - Acronyms and mixed-case words (e.g., "iPhone", "McDonald")[9].
    - Languages with complex punctuation rules or continuous scripts[9].
    - Real-time streaming, where context is limited.

- **Recent Advances:**  
  - Training on longer, sentence-complete segments (rather than short, partial segments) improves PnC accuracy and overall ASR performance, especially for long-form audio[5].
  - End-to-end models that predict PnC directly from audio (rather than as a post-processing step) are emerging, but cascaded approaches (ASR → PnC) remain common in production[8].

---

### **Example: Using Canary Model for Streaming ASR with PnC**

The **Canary** model in NeMo supports streaming ASR with optional punctuation and capitalization output. You can specify the `"pnc": "yes"` flag in the input manifest to request PnC in the output[8]:

```python
from nemo.collections.asr.models import EncDecMultiTaskModel

canary_model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b')

predicted_text = canary_model.transcribe(
    "<path to input manifest file>",
    batch_size=16,
)[0].text
```
- The output will include punctuation and capitalization if requested[8].

---

### **Summary Table: NeMo PnC Model Inference**

| Scenario                | Input                | Output                        | Notes                                      |
|-------------------------|----------------------|-------------------------------|---------------------------------------------|
| Offline/Batch           | ASR text (manifest)  | Punctuated, capitalized text  | Use script or API; sliding window for long  |
| Streaming (chunked)     | ASR text (chunks)    | Punctuated, capitalized text  | Buffering required; latency/accuracy trade  |
| End-to-end (Canary)     | Audio + PnC flag     | Punctuated, capitalized text  | Direct output from model if supported       |

---

### **References to Key Implementation Details**

- **Official NeMo Docs:**  
  [2][4] for model types, data format, and inference scripts.
- **Canary Model Usage:**  
  [8] for streaming ASR with PnC output.
- **Streaming ASR Output:**  
  [6] for FastConformer streaming output (no PnC by default).
- **Performance and Limitations:**  
  [5][9] for accuracy, limitations, and recent research.

---

**In summary:**  
NeMo PnC models can be used in both offline and streaming ASR pipelines to restore punctuation and capitalization, either as a post-processing step or directly within end-to-end models like Canary. For streaming, buffering and chunk-wise inference are typical, with a trade-off between latency and accuracy. The models are typically based on pre-trained language models with joint token-level classification heads, and are integrated via scripts or API calls in the NeMo framework[2][4][8].

## Citations

### 1. Models — NVIDIA NeMo Framework User Guide

**URL:** https://docs.nvidia.com/nemo-framework/user-guide/latest/nemotoolkit/asr/models.html

# Models#

This section gives a brief overview of the models that NeMo’s ASR collection currently supports.

Each of these models can be used with the example ASR scripts (in the

`<NeMo_git_root>/examples/asr` directory) by

specifying the model architecture in the config file used. Examples of config files for each model can be found in

the

`<NeMo_git_root>/examples/asr/conf` directory.

For more information about the config files and how they should be structured, refer to the section.

Pretrained checkpoints for all of these models, as well as instructions on how to load them, can be found in the section. You can use the available checkpoints for immediate inference, or fine-tune them on your own datasets. The checkpoints section also contains benchmark results for the available ASR models.

## Spotlight Models#

### Canary#

Canary is the latest family of models from NVIDIA NeMo. Canary models are encoder-decoder models with a and Transformer Decoder []. They are multi-lingual, multi-task model, supporting automatic speech-to-text recognition (ASR) in 25 EU languages as well as translation between English and the 24 other supported languages.

Models:

model card

model card

model card

model card

Spaces:

Canary models support the following decoding methods for chunked and streaming inference:

### Parakeet#

Parakeet is the name of a family of ASR models with a and a CTC, RNN-T, or TDT decoder.

Model checkpoints:

model card

this model sits top of the at time of writing (May 2nd 2025)



and model cards

and model cards

model card

HuggingFace Spaces to try out Parakeet models in your browser:

space... ### Conformer-HAT#

Conformer HAT (Hybrid Autoregressive Transducer) model (do not confuse it with Hybrid-Transducer-CTC) is a modification of Conformer-Transducer model based on this previous . The main idea is to separate labels and blank score predictions, which allows to estimate the internal LM probabilities during decoding. When external LM is available for inference, the internal LM can be subtracted from HAT model prediction in beamsearch decoding to improve external LM efficiency. It can be helpful in the case of text-only adaptation for new domains.

The only difference from the standard Conformer-Transducer model (RNNT) is the use of class (instead of “RNNTJoint”) for joint module. The all HAT logic is implemented in the “HATJoint” class.

You may find the example config files of Conformer-HAT model with character-based encoding at

`<NeMo_git_root>/examples/asr/conf/conformer/hat/conformer_hat_char.yaml` and

with sub-word encoding at

`<NeMo_git_root>/examples/asr/conf/conformer/hat/conformer_hat_bpe.yaml`.

By default, the decoding for HAT model works in the same way as for Conformer-Transducer.

In the case of external ngram LM fusion you can use

`<NeMo_git_root>/scripts/asr_language_modeling/ngram_lm/eval_beamsearch_ngram_transducer.py`.

To enable HAT internal LM subtraction set

`hat_subtract_ilm=True` and find more appropriate couple of

`beam_alpha` and

`hat_ilm_weight` values in terms of the best recognition accuracy.... ## Fast-Conformer#

The Fast Conformer (CTC and RNNT) models have a faster version of the Conformer encoder and differ from it as follows:

8x depthwise convolutional subsampling with 256 channels

Reduced convolutional kernel size of 9 in the conformer blocks

The Fast Conformer encoder is about 2.4x faster than the regular Conformer encoder without a significant model quality degradation. 128 subsampling channels yield a 2.7x speedup vs baseline but model quality starts to degrade. With local attention, inference is possible on audios >1 hrs (256 subsampling channels) / >2 hrs (128 channels).

Fast Conformer models were trained using CosineAnnealing (instead of Noam) as the scheduler.

You may find the example CTC config at

`<NeMo_git_root>/examples/asr/conf/fastconformer/fast-conformer_ctc_bpe.yaml` and

the transducer config at

`<NeMo_git_root>/examples/asr/conf/fastconformer/fast-conformer_transducer_bpe.yaml`

Note that both configs are subword-based (BPE).

You can also train these models with longformer-style attention () using the following configs: CTC config at

`<NeMo_git_root>/examples/asr/conf/fastconformer/fast-conformer-long_ctc_bpe.yaml` and transducer config at

`<NeMo_git_root>/examples/asr/conf/fastconformer/fast-conformer-long_transducer_bpe.yaml`

This allows using the model on longer audio (up to 70 minutes with Fast Conformer). Note that the Fast Conformer checkpoints

can be used with limited context attention even if trained with full context. However, if you also want to use global tokens,

which help aggregate information from outside the limited context, then training is required.

You may find more examples under

`<NeMo_git_root>/examples/asr/conf/fastconformer/`.... ## Cache-aware Streaming Conformer#

Try real-time ASR with the .

Buffered streaming uses overlapping chunks to make an offline ASR model usable for streaming with reasonable accuracy. However, it causes significant amount of duplication in computation due to the overlapping chunks. Also, there is an accuracy gap between the offline model and the streaming one, as there is inconsistency between how we train the model and how we perform inference for streaming. The Cache-aware Streaming Conformer models tackle and address these disadvantages. These streaming Conformers are trained with limited right context, making it possible to match how the model is being used in both training and inference. They also use caching to store intermediate activations to avoid any duplication in compute. The cache-aware approach is supported for both the Conformer-CTC and Conformer-Transducer and enables the model to be used very efficiently for streaming.

Three categories of layers in Conformer have access to right tokens: #. depthwise convolutions #. self-attention #. convolutions in the downsampling layers.

Streaming Conformer models use causal convolutions or convolutions with lower right context and also self-attention with limited right context to limit the effective right context for the input. The model trained with such limitations can be used in streaming mode and give the exact same outputs and accuracy as when the whole audio is given to the model in offline mode. These model can use caching mechanism to store and reuse the activations during streaming inference to avoid any duplications in the computations as much as possible.... at

`<NeMo_git_root>/examples/asr/conf/conformer/cache_aware_streaming/conformer_ctc_bpe.yaml` for CTC variant. It is recommended to use FastConformer as they are more than 2X faster in both training and inference than regular Conformer.

The hybrid versions of FastConformer can be found here:

`<NeMo_git_root>/examples/asr/conf/conformer/hybrid_cache_aware_streaming/`

Examples for regular Conformer can be found at

`<NeMo_git_root>/examples/asr/conf/conformer/cache_aware_streaming/conformer_transducer_bpe_streaming.yaml` for Transducer variant and

at

`<NeMo_git_root>/examples/asr/conf/conformer/cache_aware_streaming/conformer_ctc_bpe.yaml` for CTC variant.

To simulate cache-aware streaming, you may use the script at

`<NeMo_git_root>/examples/asr/asr_cache_aware_streaming/speech_to_text_cache_aware_streaming_infer.py`. It can simulate streaming in single stream or multi-stream mode (in batches) for an ASR model.

This script can be used for models trained offline with full-context but the accuracy would not be great unless the chunk size is large enough which would result in high latency.... ## LSTM-Transducer#

LSTM-Transducer is a model which uses RNNs (eg. LSTM) in the encoder. The architecture of this model is followed from suggestions in []. It uses RNNT/Transducer loss/decoder. The encoder consists of RNN layers (LSTM as default) with lower projection size to increase the efficiency. Layer norm is added between the layers to stabilize the training. It can be trained/used in unidirectional or bidirectional mode. The unidirectional mode is fully causal and can be used easily for simple and efficient streaming. However the accuracy of this model is generally lower than other models like Conformer and Citrinet.

This model supports both the sub-word level and character level encodings. You may find the example config file of RNNT model with wordpiece encoding at

`<NeMo_git_root>/examples/asr/conf/lstm/lstm_transducer_bpe.yaml`.

You can find more details on the config files for the RNNT models at .

## LSTM-CTC#

LSTM-CTC model is a CTC-variant of the LSTM-Transducer model which uses CTC loss/decoding instead of Transducer.

You may find the example config file of LSTM-CTC model with wordpiece encoding at

`<NeMo_git_root>/examples/asr/conf/lstm/lstm_ctc_bpe.yaml`.

### 2. Cache aware streaming ASR - discrepancies between training and inference · NVIDIA NeMo · Discussion #7010

**URL:** https://github.com/NVIDIA/NeMo/discussions/7010

# Cache aware streaming ASR - discrepancies between training and inference #7010

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

## tobygodwin Jul 11, 2023

Original comment in English -

Hello,

Firstly thanks for putting together a great tool.

I've trained a cache aware streaming fastconformer model using the standard config and the standard ctc asr training script .

The model trains well, but when I go to evaluate the streaming performance using the cache aware streaming inference script there is a drastic difference between offline and streaming performance (13% and 70% respectively), which surprised me because I thought that I had limited the attention context using
```
chunked_limited
```
during training, and that if this was matched during inference then equivalent performance would be achieved. When I increase the --chunk_size during streaming inference, performance improves significantly to 19% which implies to me that the original model was trained without any restrictions on attention context.

I've confirmed that the default streaming configs are the same between training and inference (the standard parameters returned by
```
setup_streaming_params()
```
for my config). One thing I did notice is that
```
cache_last_channel
```
and
```
cache_last_time
```
are not being passed during the training forward pass, but they are during the streaming inference forward pass. I guess this results in a mismatch between training and inference which would lead to differences in performance.

Am I missing something? I would have thought that
```
cache_last_channel
```
etc should be included in the training procedures.

Any guidance would be much appreciated.

Many thanks

Beta Was this translation helpful? Give feedback.

All reactions

Answered by VahidooX Jul 14, 2023

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

View full answer... ## Replies: 5 comments 2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 11, 2023 Collaborator

-

Which branch or nemo version are you using? Caches are not need during training. They are being used during inference as whole input is not given at once. You should get the exact same accuracy with the streaming script. We have made some changes recently. Let me check everything and get back to you.

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 13, 2023 Author

-

thanks for your response.

I'm using the master branch on this commit:

52d3af7

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 14, 2023 Collaborator

-

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

Answer selected by tobygodwin

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 18, 2023 Author

-

Sorry for the delay. That worked perfectly! Thanks very much

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### sangeet2020 Jun 11, 2024

-

Hello @titu1994 ,

I am in the process of training this model for German language, and I obtained these training stats after 100 epochs.

Looking at the training loss and train wer, I believe the losses have not yet converged. Do you think something is wrong with the parameters/hyper-params. or does the training needs to continue?

Thank You

Beta Was this translation helpful? Give feedback.

All reactions

2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### titu1994 Jun 11, 2024 Maintainer

Original comment in English -

Please don't comment on old discussions, open a new issue, since your discussion didn't seem related to this one.

In any case, it looks like youre convergence is slow. Try using a pretrained model to speed it up. If it's chunk aware specifically, then training longer will help)

Beta Was this translation helpful? Give feedback.

All reactions

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### sangeet2020 Jun 11, 2024

Original comment in English -

Apologies. Will do so. thank you

Beta Was this translation helpful? Give feedback.

All reactions

Category

Q&A

Labels

None yet

4 participants

### 3. Cache aware streaming ASR - discrepancies between training and inference · NVIDIA NeMo · Discussion #7010

**URL:** https://github.com/NVIDIA/NeMo/discussions/7010

# Cache aware streaming ASR - discrepancies between training and inference #7010

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

## tobygodwin Jul 11, 2023

Original comment in English -

Hello,

Firstly thanks for putting together a great tool.

I've trained a cache aware streaming fastconformer model using the standard config and the standard ctc asr training script .

The model trains well, but when I go to evaluate the streaming performance using the cache aware streaming inference script there is a drastic difference between offline and streaming performance (13% and 70% respectively), which surprised me because I thought that I had limited the attention context using
```
chunked_limited
```
during training, and that if this was matched during inference then equivalent performance would be achieved. When I increase the --chunk_size during streaming inference, performance improves significantly to 19% which implies to me that the original model was trained without any restrictions on attention context.

I've confirmed that the default streaming configs are the same between training and inference (the standard parameters returned by
```
setup_streaming_params()
```
for my config). One thing I did notice is that
```
cache_last_channel
```
and
```
cache_last_time
```
are not being passed during the training forward pass, but they are during the streaming inference forward pass. I guess this results in a mismatch between training and inference which would lead to differences in performance.

Am I missing something? I would have thought that
```
cache_last_channel
```
etc should be included in the training procedures.

Any guidance would be much appreciated.

Many thanks

Beta Was this translation helpful? Give feedback.

All reactions

Answered by VahidooX Jul 14, 2023

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

View full answer... ## Replies: 5 comments 2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 11, 2023 Collaborator

-

Which branch or nemo version are you using? Caches are not need during training. They are being used during inference as whole input is not given at once. You should get the exact same accuracy with the streaming script. We have made some changes recently. Let me check everything and get back to you.

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 13, 2023 Author

-

thanks for your response.

I'm using the master branch on this commit:

52d3af7

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 14, 2023 Collaborator

-

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

Answer selected by tobygodwin

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 18, 2023 Author

-

Sorry for the delay. That worked perfectly! Thanks very much

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### sangeet2020 Jun 11, 2024

-

Hello @titu1994 ,

I am in the process of training this model for German language, and I obtained these training stats after 100 epochs.

Looking at the training loss and train wer, I believe the losses have not yet converged. Do you think something is wrong with the parameters/hyper-params. or does the training needs to continue?

Thank You

Beta Was this translation helpful? Give feedback.

All reactions

2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### titu1994 Jun 11, 2024 Maintainer

Original comment in English -

Please don't comment on old discussions, open a new issue, since your discussion didn't seem related to this one.

In any case, it looks like youre convergence is slow. Try using a pretrained model to speed it up. If it's chunk aware specifically, then training longer will help)

Beta Was this translation helpful? Give feedback.

All reactions

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### sangeet2020 Jun 11, 2024

Original comment in English -

Apologies. Will do so. thank you

Beta Was this translation helpful? Give feedback.

All reactions

Category

Q&A

Labels

None yet

4 participants

### 4. nvidia/canary-1b - Hugging Face

**URL:** https://huggingface.co/nvidia/canary-1b

## NVIDIA NeMo

To train, fine-tune or Transcribe with Canary, you will need to install NVIDIA NeMo. We recommend you install it after you've installed Cython and latest PyTorch version.

```

pip install git+https://github.com/NVIDIA/NeMo.git@r1.23.0#egg=nemo_toolkit[asr]

```... ## How to Use this Model

The model is available for use in the NeMo toolkit [4], and can be used as a pre-trained checkpoint for inference or for fine-tuning on another dataset.

### Loading the Model

```

from nemo.collections.asr.models import EncDecMultiTaskModel

# load model

canary_model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b')

# update dcode params

decode_cfg = canary_model.cfg.decoding

decode_cfg.beam.beam_size = 1

canary_model.change_decoding_strategy(decode_cfg)

```

### Input Format

Input to Canary can be either a list of paths to audio files or a jsonl manifest file.

If the input is a list of paths, Canary assumes that the audio is English and Transcribes it. I.e., Canary default behaviour is English ASR.

```

predicted_text = canary_model.transcribe(

paths2audio_files=['path1.wav', 'path2.wav'],

batch_size=16, # batch size to run the inference with

)[0].text

```

To use Canary for transcribing other supported languages or perform Speech-to-Text translation, specify the input as jsonl manifest file, where each line in the file is a dictionary containing the following fields:

```

# Example of a line in input_manifest.json



"audio_filepath": "/path/to/audio.wav", # path to the audio file

"duration": 1000, # duration of the audio, can be set to `None` if using NeMo main branch

"taskname": "asr", # use "s2t_translation" for speech-to-text translation with r1.23, or "ast" if using the NeMo main branch

"source_lang": "en", # language of the audio input, set `source_lang`==`target_lang` for ASR, choices=['en','de','es','fr']

"target_lang": "en", # language of the text output, choices=['en','de','es','fr']

"pnc": "yes", # whether to have PnC output, choices=['yes', 'no']

"answer": "na",



```

and then use:

```

predicted_text = canary_model.transcribe(

"<path to input manifest file>",

batch_size=16, # batch size to run the inference with

)[0].text

```... ### Input

This model accepts single channel (mono) audio sampled at 16000 Hz, along with the task/languages/PnC tags as input.

### Output

The model outputs the transcribed/translated text corresponding to the input audio, in the specified target language and with or without punctuation and capitalization.

## Training

Canary-1B is trained using the NVIDIA NeMo toolkit [4] for 150k steps with dynamic bucketing and a batch duration of 360s per GPU on 128 NVIDIA A100 80GB GPUs. The model can be trained using this example script and base config.

The tokenizers for these models were built using the text transcripts of the train set with this script.

### Datasets

The Canary-1B model is trained on a total of 85k hrs of speech data. It consists of 31k hrs of public data, 20k hrs collected by Suno, and 34k hrs of in-house data.

The constituents of public data are as follows.

#### English (25.5k hours)

- Librispeech 960 hours

- Fisher Corpus

- Switchboard-1 Dataset

- WSJ-0 and WSJ-1

- National Speech Corpus (Part 1, Part 6)

- VCTK

- VoxPopuli (EN)

- Europarl-ASR (EN)

- Multilingual Librispeech (MLS EN) - 2,000 hour subset

- Mozilla Common Voice (v7.0)

- People's Speech - 12,000 hour subset

- Mozilla Common Voice (v11.0) - 1,474 hour subset

#### German (2.5k hours)

- Mozilla Common Voice (v12.0) - 800 hour subset

- Multilingual Librispeech (MLS DE) - 1,500 hour subset

- VoxPopuli (DE) - 200 hr subset

#### Spanish (1.4k hours)

- Mozilla Common Voice (v12.0) - 395 hour subset

- Multilingual Librispeech (MLS ES) - 780 hour subset

- VoxPopuli (ES) - 108 hour subset

- Fisher - 141 hour subset

#### French (1.8k hours)

- Mozilla Common Voice (v12.0) - 708 hour subset

- Multilingual Librispeech (MLS FR) - 926 hour subset

- VoxPopuli (FR) - 165 hour subset... ## Performance

In both ASR and AST experiments, predictions were generated using beam search with width 5 and length penalty 1.0.

### ASR Performance (w/o PnC)

The ASR performance is measured with word error rate (WER), and we process the groundtruth and predicted text with whisper-normalizer.

WER on MCV-16.1 test set:

|Version|Model|En|De|Es|Fr|
|--|--|--|--|--|--|
|1.23.0|canary-1b|7.97|4.61|3.99|6.53|
WER on MLS test set:

|Version|Model|En|De|Es|Fr|
|--|--|--|--|--|--|
|1.23.0|canary-1b|3.06|4.19|3.15|4.12|
More details on evaluation can be found at HuggingFace ASR Leaderboard

### AST Performance

We evaluate AST performance with BLEU score, and use native annotations with punctuation and capitalization in the datasets.

BLEU score on FLEURS test set:

|Version|Model|En->De|En->Es|En->Fr|De->En|Es->En|Fr->En|
|--|--|--|--|--|--|--|--|
|1.23.0|canary-1b|32.15|22.66|40.76|33.98|21.80|30.95|
BLEU score on COVOST-v2 test set:

|Version|Model|De->En|Es->En|Fr->En|
|--|--|--|--|--|
|1.23.0|canary-1b|37.67|40.7|40.42|
BLEU score on mExpresso test set:

|Version|Model|En->De|En->Es|En->Fr|
|--|--|--|--|--|
|1.23.0|canary-1b|23.84|35.74|28.29|... ## Model Fairness Evaluation

As outlined in the paper "Towards Measuring Fairness in AI: the Casual Conversations Dataset", we assessed the Canary-1B model for fairness. The model was evaluated on the CausalConversations-v1 dataset, and the results are reported as follows:

### Gender Bias:

|Gender|Male|Female|N/A|Other|
|--|--|--|--|--|
|Num utterances|19325|24532|926|33|
|% WER|14.64|12.92|17.88|126.92|
### Age Bias:

|Age Group|(18-30)|(31-45)|(46-85)|(1-100)|
|--|--|--|--|--|
|Num utterances|15956|14585|13349|43890|
|% WER|14.64|13.07|13.47|13.76|
(Error rates for fairness evaluation are determined by normalizing both the reference and predicted text, similar to the methods used in the evaluations found at https://github.com/huggingface/open_asr_leaderboard.)

## NVIDIA Riva: Deployment

NVIDIA Riva, is an accelerated speech AI SDK deployable on-prem, in all clouds, multi-cloud, hybrid, on edge, and embedded. Additionally, Riva provides:

- World-class out-of-the-box accuracy for the most common languages with model checkpoints trained on proprietary data with hundreds of thousands of GPU-compute hours

- Best in class accuracy with run-time word boosting (e.g., brand and product names) and customization of acoustic model, language model, and inverse text normalization

- Streaming speech recognition, Kubernetes compatible scaling, and enterprise-grade support

Canary is available as a NIM endpoint via Riva. Try the model yourself here: https://build.nvidia.com/nvidia/canary-1b-asr.

### 5. Cache aware streaming ASR - discrepancies between training and inference · NVIDIA NeMo · Discussion #7010

**URL:** https://github.com/NVIDIA/NeMo/discussions/7010

# Cache aware streaming ASR - discrepancies between training and inference #7010

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

## tobygodwin Jul 11, 2023

Original comment in English -

Hello,

Firstly thanks for putting together a great tool.

I've trained a cache aware streaming fastconformer model using the standard config and the standard ctc asr training script .

The model trains well, but when I go to evaluate the streaming performance using the cache aware streaming inference script there is a drastic difference between offline and streaming performance (13% and 70% respectively), which surprised me because I thought that I had limited the attention context using
```
chunked_limited
```
during training, and that if this was matched during inference then equivalent performance would be achieved. When I increase the --chunk_size during streaming inference, performance improves significantly to 19% which implies to me that the original model was trained without any restrictions on attention context.

I've confirmed that the default streaming configs are the same between training and inference (the standard parameters returned by
```
setup_streaming_params()
```
for my config). One thing I did notice is that
```
cache_last_channel
```
and
```
cache_last_time
```
are not being passed during the training forward pass, but they are during the streaming inference forward pass. I guess this results in a mismatch between training and inference which would lead to differences in performance.

Am I missing something? I would have thought that
```
cache_last_channel
```
etc should be included in the training procedures.

Any guidance would be much appreciated.

Many thanks

Beta Was this translation helpful? Give feedback.

All reactions

Answered by VahidooX Jul 14, 2023

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

View full answer... ## Replies: 5 comments 2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 11, 2023 Collaborator

-

Which branch or nemo version are you using? Caches are not need during training. They are being used during inference as whole input is not given at once. You should get the exact same accuracy with the streaming script. We have made some changes recently. Let me check everything and get back to you.

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 13, 2023 Author

-

thanks for your response.

I'm using the master branch on this commit:

52d3af7

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### VahidooX Jul 14, 2023 Collaborator

-

I found a bug in updating the caches and fixed it in this PR: #7034

Would you please try it out?

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

Answer selected by tobygodwin

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

### tobygodwin Jul 18, 2023 Author

-

Sorry for the delay. That worked perfectly! Thanks very much

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### sangeet2020 Jun 11, 2024

-

Hello @titu1994 ,

I am in the process of training this model for German language, and I obtained these training stats after 100 epochs.

Looking at the training loss and train wer, I believe the losses have not yet converged. Do you think something is wrong with the parameters/hyper-params. or does the training needs to continue?

Thank You

Beta Was this translation helpful? Give feedback.

All reactions

2 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-
- ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### titu1994 Jun 11, 2024 Maintainer

Original comment in English -

Please don't comment on old discussions, open a new issue, since your discussion didn't seem related to this one.

In any case, it looks like youre convergence is slow. Try using a pretrained model to speed it up. If it's chunk aware specifically, then training longer will help)

Beta Was this translation helpful? Give feedback.

All reactions

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### sangeet2020 Jun 11, 2024

Original comment in English -

Apologies. Will do so. thank you

Beta Was this translation helpful? Give feedback.

All reactions

Category

Q&A

Labels

None yet

4 participants

## Metadata

```json
{
  "planId": "plan_4",
  "executionTime": 57286,
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
