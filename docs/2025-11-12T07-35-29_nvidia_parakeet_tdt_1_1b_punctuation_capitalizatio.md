# Search Results

**Query:** NVIDIA Parakeet-TDT 1.1B punctuation capitalization modes automatic formatting
**Date:** 2025-11-12T07:35:29.505Z

## Answer

**NVIDIA Parakeet-TDT 1.1B** offers two main modes for handling punctuation and capitalization in automatic speech recognition (ASR):  
- **parakeet-tdt-1.1b**: outputs all text in lower case, with no punctuation or capitalization.  
- **parakeet-tdt_ctc-1.1b**: outputs text with **automatic punctuation and capitalization** for the English alphabet, formatting the transcript to be more readable and suitable for downstream tasks[1][2][3][7].

---

### **Punctuation and Capitalization Modes**

| Model Variant                | Output Formatting                  | Automatic Punctuation | Automatic Capitalization |
|------------------------------|------------------------------------|----------------------|-------------------------|
| **parakeet-tdt-1.1b**        | Lowercase only                     | No                   | No                      |
| **parakeet-tdt_ctc-1.1b**    | Punctuated, capitalized sentences  | Yes                  | Yes                     |

- **parakeet-tdt_ctc-1.1b** is specifically trained to insert punctuation marks (such as commas, periods, question marks) and apply correct capitalization (e.g., sentence starts, proper nouns) in the output transcript automatically[1][7].
- **parakeet-tdt-1.1b** is intended for use cases where raw, unformatted text is preferred, such as further downstream NLP processing or custom formatting[2].

---

### **Automatic Formatting Details**

- **Input**: Both models accept 16kHz mono-channel WAV audio files.
- **Output**:
  - **parakeet-tdt_ctc-1.1b**: Returns a string with punctuation and capitalization applied, closely resembling standard written English[1][7].
  - **parakeet-tdt-1.1b**: Returns a string in all lowercase, with no punctuation[2][3].

**Example usage for automatic formatting** (Python, using NeMo toolkit):
```python
import nemo.collections.asr as nemo_asr
asr_model = nemo_asr.models.ASRModel.from_pretrained(model_name="nvidia/parakeet-tdt_ctc-1.1b")
output = asr_model.transcribe(['your_audio.wav'])
print(output[0].text)  # Output will include punctuation and capitalization
```
[1][7]

---

### **How Automatic Formatting Works**

- The **punctuation and capitalization** are predicted by the model as part of the decoding process, not as a post-processing step. This is achieved by training the model on large, punctuated, and capitalized English datasets, allowing it to learn where to insert punctuation marks and when to capitalize words[1][7].
- The model is designed to make the output more readable and directly usable for applications such as transcription, subtitles, and documentation[1][7].

---

### **Limitations and Customization**

- The model is optimized for **English** and may not generalize to other languages or scripts.
- Customization (e.g., adding support for different punctuation marks or capitalization rules) would require further fine-tuning or retraining on appropriately labeled data[1][7].
- For advanced customization, NVIDIA NeMo provides tools to fine-tune or adapt the model to specific domains or formatting requirements[1][7].

---

### **Summary Table**

| Feature                     | parakeet-tdt-1.1b | parakeet-tdt_ctc-1.1b |
|-----------------------------|-------------------|-----------------------|
| Punctuation                 | No                | Yes                   |
| Capitalization              | No                | Yes                   |
| Output Example              | "hello world"     | "Hello, world."       |
| Use Case                    | Raw ASR/NLP       | Readable transcripts  |

---

**In summary:**  
- Use **parakeet-tdt_ctc-1.1b** for **automatic punctuation and capitalization** in English ASR output.
- Use **parakeet-tdt-1.1b** for **lowercase, unformatted** output[1][2][7].  
Both models are accessible via the NVIDIA NeMo toolkit and can be selected according to your formatting needs.  

[1]: https://dataloop.ai/library/model/nvidia_parakeet-tdt_ctc-11b/  
[2]: https://dataloop.ai/library/model/nvidia_parakeet-tdt-11b/  
[3]: https://fxis.ai/edu/how-to-utilize-the-parakeet-tdt-1-1b-asr-model/  
[7]: https://model.aibase.com/models/details/1915693347899072514

## Citations

### 1. nvidia/parakeet-tdt_ctc-1.1b - Hugging Face

**URL:** https://huggingface.co/nvidia/parakeet-tdt_ctc-1.1b

# Parakeet TDT-CTC 1.1B PnC(en)

`parakeet-tdt_ctc-1.1b` is an ASR model that transcribes speech with Punctuations and Capitalizations of English alphabet. This model is jointly developed by NVIDIA NeMo and Suno.ai teams.

It is an XXL version of Hybrid FastConformer [1] TDT-CTC [2] (around 1.1B parameters) model. This model has been trained with Local Attention and Global token hence this model can transcribe

**11 hrs** of audio in one single pass. And for reference this model can transcibe 90mins of audio in <16 sec on A100.

See the model architecture section and NeMo documentation for complete architecture details.

## NVIDIA NeMo: Training

To train, fine-tune or play with the model you will need to install NVIDIA NeMo. We recommend you install it after you've installed latest PyTorch version.

```

pip install nemo_toolkit['all']

```... ## How to Use this Model

The model is available for use in the NeMo toolkit [3], and can be used as a pre-trained checkpoint for inference or for fine-tuning on another dataset.

### Automatically instantiate the model

```

import nemo.collections.asr as nemo_asr

asr_model = nemo_asr.models.ASRModel.from_pretrained(model_name="nvidia/parakeet-tdt_ctc-1.1b")

```

### Transcribing using Python

First, let's get a sample

```

wget https://dldata-public.s3.us-east-2.amazonaws.com/2086-149220-0033.wav

```

Then simply do:

```

output = asr_model.transcribe(['2086-149220-0033.wav'])

print(output[0].text)

```

### Transcribing many audio files

By default model uses TDT to transcribe the audio files, to switch decoder to use CTC, use decoding_type='ctc'

```

python [NEMO_GIT_FOLDER]/examples/asr/transcribe_speech.py

pretrained_name="nvidia/parakeet-tdt_ctc-1.1b"

audio_dir="<DIRECTORY CONTAINING AUDIO FILES>"

```

### Input

This model accepts 16000 Hz mono-channel audio (wav files) as input.

### Output

This model provides transcribed speech as a string for a given audio sample.... ## Model Architecture

This model uses a Hybrid FastConformer-TDT-CTC architecture. FastConformer [1] is an optimized version of the Conformer model with 8x depthwise-separable convolutional downsampling. You may find more information on the details of FastConformer here: Fast-Conformer Model.

## Training

The NeMo toolkit [3] was used for finetuning this model for 20,000 steps over

`parakeet-tdt-1.1` model. This model is trained with this example script and this base config.

The tokenizers for these models were built using the text transcripts of the train set with this script.

### Datasets

The model was trained on 36K hours of English speech collected and prepared by NVIDIA NeMo and Suno teams.

The training dataset consists of private subset with 27K hours of English speech plus 9k hours from the following public PnC datasets:

- Librispeech 960 hours of English speech

- Fisher Corpus

- National Speech Corpus Part 1

- VCTK

- VoxPopuli (EN)

- Europarl-ASR (EN)

- Multilingual Librispeech (MLS EN) - 2,000 hour subset

- Mozilla Common Voice (v7.0)... ## Performance

The performance of Automatic Speech Recognition models is measuring using Word Error Rate. Since this dataset is trained on multiple domains and a much larger corpus, it will generally perform better at transcribing audio in general.

The following tables summarizes the performance of the available models in this collection with the Transducer decoder. Performances of the ASR models are reported in terms of Word Error Rate (WER%) with greedy decoding.

|Version|Tokenizer|Vocabulary Size|AMI|Earnings-22|Giga Speech|LS test-clean|LS test-other|SPGI Speech|TEDLIUM-v3|Vox Populi|Common Voice|
|--|--|--|--|--|--|--|--|--|--|--|--|
|1.23.0|SentencePiece Unigram|1024|15.94|11.86|10.19|1.82|3.67|2.24|3.87|6.19|8.69|
These are greedy WER numbers without external LM. More details on evaluation can be found at HuggingFace ASR Leaderboard

## Model Fairness Evaluation

As outlined in the paper "Towards Measuring Fairness in AI: the Casual Conversations Dataset", we assessed the parakeet-tdt_ctc-1.1b model for fairness. The model was evaluated on the CausalConversations-v1 dataset, and the results are reported as follows:

### Gender Bias:

|Gender|Male|Female|N/A|Other|
|--|--|--|--|--|
|Num utterances|19325|24532|926|33|
|% WER|12.81|10.49|13.88|23.12|
### Age Bias:

|Age Group|(18-30)|(31-45)|(46-85)|(1-100)|
|--|--|--|--|--|
|Num utterances|15956|14585|13349|43890|
|% WER|11.50|11.63|11.38|11.51|
(Error rates for fairness evaluation are determined by normalizing both the reference and predicted text, similar to the methods used in the evaluations found at https://github.com/huggingface/open_asr_leaderboard.)... ## NVIDIA Riva: Deployment

NVIDIA Riva, is an accelerated speech AI SDK deployable on-prem, in all clouds, multi-cloud, hybrid, on edge, and embedded. Additionally, Riva provides:

- World-class out-of-the-box accuracy for the most common languages with model checkpoints trained on proprietary data with hundreds of thousands of GPU-compute hours

- Best in class accuracy with run-time word boosting (e.g., brand and product names) and customization of acoustic model, language model, and inverse text normalization

- Streaming speech recognition, Kubernetes compatible scaling, and enterprise-grade support

Although this model isn’t supported yet by Riva, the list of supported models is here.

Check out Riva live demo.

## References

[1] Fast Conformer with Linearly Scalable Attention for Efficient Speech Recognition

[2] Efficient Sequence Transduction by Jointly Predicting Tokens and Durations

[3] Google Sentencepiece Tokenizer

[5] Suno.ai

[6] HuggingFace ASR Leaderboard

[7] Towards Measuring Fairness in AI: the Casual Conversations Dataset

## Licence

License to use this model is covered by the CC-BY-4.0. By downloading the public and release version of the model, you accept the terms and conditions of the CC-BY-4.0 license.

- Downloads last month

- 26,698... ## Model tree for nvidia/parakeet-tdt_ctc-1.1b

## Datasets used to train nvidia/parakeet-tdt_ctc-1.1b

## Space using nvidia/parakeet-tdt_ctc-1.1b 1

## Collection including nvidia/parakeet-tdt_ctc-1.1b

## Evaluation results

- Test WER on AMI (Meetings test)test set self-reported15.940

- Test WER on Earnings-22test set self-reported11.860

- Test WER on GigaSpeechtest set self-reported10.190

- Test WER on LibriSpeech (clean)test set self-reported1.820

- Test WER on LibriSpeech (other)test set self-reported3.670

- Test WER on SPGI Speechtest set self-reported2.240

- Test WER on tedlium-v3test set self-reported3.870

- Test WER on Vox Populitest set self-reported6.190

- Test WER on Mozilla Common Voice 9.0test set self-reported8.690

### 2. nvidia/parakeet-tdt-1.1b - Hugging Face

**URL:** https://huggingface.co/nvidia/parakeet-tdt-1.1b

# Parakeet TDT 1.1B (en)

`parakeet-tdt-1.1b` is an ASR model that transcribes speech in lower case English alphabet. This model is jointly developed by NVIDIA NeMo and Suno.ai teams.

It is an XXL version of FastConformer [1] TDT [2] (around 1.1B parameters) model.

See the model architecture section and NeMo documentation for complete architecture details.

## NVIDIA NeMo: Training

To train, fine-tune or play with the model you will need to install NVIDIA NeMo. We recommend you install it after you've installed latest PyTorch version.

```

pip install nemo_toolkit['all']

```

## How to Use this Model

The model is available for use in the NeMo toolkit [3], and can be used as a pre-trained checkpoint for inference or for fine-tuning on another dataset.

### Automatically instantiate the model

```

import nemo.collections.asr as nemo_asr

asr_model = nemo_asr.models.EncDecRNNTBPEModel.from_pretrained(model_name="nvidia/parakeet-tdt-1.1b")

```

### Transcribing using Python

First, let's get a sample

```

wget https://dldata-public.s3.us-east-2.amazonaws.com/2086-149220-0033.wav

```

Then simply do:

```

output = asr_model.transcribe(['2086-149220-0033.wav'])

print(output[0].text)

```

### Transcribing many audio files

```

python [NEMO_GIT_FOLDER]/examples/asr/transcribe_speech.py

pretrained_name="nvidia/parakeet-tdt-1.1b"

audio_dir="<DIRECTORY CONTAINING AUDIO FILES>"

```

### Input

This model accepts 16000 Hz mono-channel audio (wav files) as input.

### Output

This model provides transcribed speech as a string for a given audio sample.... ## Model Architecture

This model uses a FastConformer-TDT architecture. FastConformer [1] is an optimized version of the Conformer model with 8x depthwise-separable convolutional downsampling. You may find more information on the details of FastConformer here: Fast-Conformer Model.

TDT (Token-and-Duration Transducer) [2] is a generalization of conventional Transducers by decoupling token and duration predictions. Unlike conventional Transducers which produces a lot of blanks during inference, a TDT model can skip majority of blank predictions by using the duration output (up to 4 frames for this parakeet-tdt-1.1b model), thus brings significant inference speed-up. The detail of TDT can be found here: Efficient Sequence Transduction by Jointly Predicting Tokens and Durations.

## Training

The NeMo toolkit [3] was used for training the models for over several hundred epochs. These model are trained with this example script and this base config.

The tokenizers for these models were built using the text transcripts of the train set with this script.

### Datasets

The model was trained on 64K hours of English speech collected and prepared by NVIDIA NeMo and Suno teams.

The training dataset consists of private subset with 40K hours of English speech plus 24K hours from the following public datasets:

- Librispeech 960 hours of English speech

- Fisher Corpus

- Switchboard-1 Dataset

- WSJ-0 and WSJ-1

- National Speech Corpus (Part 1, Part 6)

- VCTK

- VoxPopuli (EN)

- Europarl-ASR (EN)

- Multilingual Librispeech (MLS EN) - 2,000 hour subset

- Mozilla Common Voice (v7.0)

- People's Speech - 12,000 hour subset... ## Performance

The performance of Automatic Speech Recognition models is measuring using Word Error Rate. Since this dataset is trained on multiple domains and a much larger corpus, it will generally perform better at transcribing audio in general.

The following tables summarizes the performance of the available models in this collection with the Transducer decoder. Performances of the ASR models are reported in terms of Word Error Rate (WER%) with greedy decoding.

|Version|Tokenizer|Vocabulary Size|AMI|Earnings-22|Giga Speech|LS test-clean|SPGI Speech|TEDLIUM-v3|Vox Populi|Common Voice|
|--|--|--|--|--|--|--|--|--|--|--|
|1.22.0|SentencePiece Unigram|1024|15.90|14.65|9.55|1.39|2.62|3.42|3.56|5.48|
These are greedy WER numbers without external LM. More details on evaluation can be found at HuggingFace ASR Leaderboard

## Model Fairness Evaluation

As outlined in the paper "Towards Measuring Fairness in AI: the Casual Conversations Dataset", we assessed the parakeet-tdt-1.1b model for fairness. The model was evaluated on the CausalConversations-v1 dataset, and the results are reported as follows:

### Gender Bias:

|Gender|Male|Female|N/A|Other|
|--|--|--|--|--|
|Num utterances|19325|24532|926|33|
|% WER|17.18|14.61|19.06|37.57|
### Age Bias:

|Age Group|$(18-30)$|$(31-45)$|$(46-85)$|$(1-100)$|
|--|--|--|--|--|
|Num utterances|15956|14585|13349|43890|
|% WER|15.83|15.89|15.46|15.74|
(Error rates for fairness evaluation are determined by normalizing both the reference and predicted text, similar to the methods used in the evaluations found at https://github.com/huggingface/open_asr_leaderboard.)... ## NVIDIA Riva: Deployment

NVIDIA Riva, is an accelerated speech AI SDK deployable on-prem, in all clouds, multi-cloud, hybrid, on edge, and embedded. Additionally, Riva provides:

- World-class out-of-the-box accuracy for the most common languages with model checkpoints trained on proprietary data with hundreds of thousands of GPU-compute hours

- Best in class accuracy with run-time word boosting (e.g., brand and product names) and customization of acoustic model, language model, and inverse text normalization

- Streaming speech recognition, Kubernetes compatible scaling, and enterprise-grade support

Although this model isn’t supported yet by Riva, the list of supported models is here.

Check out Riva live demo.

## References

[1] Fast Conformer with Linearly Scalable Attention for Efficient Speech Recognition

[2] Efficient Sequence Transduction by Jointly Predicting Tokens and Durations

[3] Google Sentencepiece Tokenizer

[5] Suno.ai

[6] HuggingFace ASR Leaderboard

[7] Towards Measuring Fairness in AI: the Casual Conversations Dataset

## Licence

License to use this model is covered by the CC-BY-4.0. By downloading the public and release version of the model, you accept the terms and conditions of the CC-BY-4.0 license.

- Downloads last month

- 2,905... ## Model tree for nvidia/parakeet-tdt-1.1b

## Datasets used to train nvidia/parakeet-tdt-1.1b

## Spaces using nvidia/parakeet-tdt-1.1b 6

## Collection including nvidia/parakeet-tdt-1.1b

## Evaluation results

- Test WER on AMI (Meetings test)test set self-reported15.900

- Test WER on Earnings-22test set self-reported14.650

- Test WER on GigaSpeechtest set self-reported9.550

- Test WER on LibriSpeech (clean)test set self-reported1.390

- Test WER on LibriSpeech (other)test set self-reported2.620

- Test WER on SPGI Speechtest set self-reported3.420

- Test WER on tedlium-v3test set self-reported3.560

- Test WER on Vox Populitest set self-reported5.480

- Test WER on Mozilla Common Voice 9.0test set self-reported5.970

### 3. On punctuation and capitalization · NVIDIA NeMo · Discussion #3819

**URL:** https://github.com/NVIDIA/NeMo/discussions/3819

# On punctuation and capitalization #3819

- ### Uh oh!

  There was an error while loading. Please reload this page.
-... ## 1-800-BAD-CODE Feb 26, 2022

Original comment in English -

|Here's a few feature requests + bugs related to the punctuation and capitalization model.
 ### Punctuation issues
 #### Inverted punctuation For languages like Spanish, we need two predictions per token to account for the possibility of inverted punctuation tokens preceding a word.
 #### Subword masking By always applying subtoken masks, continuous-script languages (e.g., Chinese) cannot be punctuated without applying some sort of pre-processing. It would be useful if the model processed text in its native script.
 #### Arbitrary punctuation tokens The text-based data set does not allow to punctuate languages such as Thai, where a space character is a punctuation token. These languages could work by having a token-based data set and removing subword masks (essentially, resolving the other issues resolves this one).
 ### Capitilization issues
 #### Acronyms The capitalization prediction is simply whether a word starts with a capital letter. So acronyms like 'amc' will not be correctly capitalized.
 #### Names that begin with a particle Similar to the acronym issue, words that begin with a particle, e.g., 'mcdonald', cannot be properly capitalized to 'McDonald'.
 #### Capitalization is independent of punctuation Currently, the two heads are conditioned only on the encoder's output and independent of each other. But capitalization is dependent on punctuation in many cases.An example of what might go wrong is we may end up with "Hello, world, What's up?" because the capitalization model might expect a period after 'world'. Essentially the capitalization head is predicting what the punctuation head will do.In practice I have found this problem to be an uncommon manifestation, but to be correct, capitalization should take into account the punctuator's output. Implicitly, we are forcing the capitalization head to learn punctuation (and predict the punctuation head's output).... ## Replies: 6 comments 3 replies

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### 1-800-BAD-CODE Mar 6, 2022 Author

-

I'm willing to implement and contribute the following solution if there is sufficient interest.

This solution should generalize to any language and solve all problems mentioned above. Given that it is vastly different from the current punctuation + capitalization model, I would propose a separate model (e.g.
```
TextStructuringModel
```
) rather than trying to change the existing model.

Here's a little bit of reasoning on which this solution is founded:

- Premise 1. Subword tokenizers should be preferred over character tokenizers.
- Premise 2. Capitalization requires a character tokenizer.
- Implication 1: Two tokenizers should be used: subword for punctuation/segmentation, character for capitalization.
- Implication 2: The model will use two language models/encoders, one for each tokenizer. [Maybe the vocab can be combined and use a single LM.]
- Premise 1. The model should generalize well to all languages and have minimal language-specific paths.
- Premise 2: It's OK to make unnecessary predictions if they are reasonably cheap.
- Implication 1: The punctuation head should always make two predictions per token, to account for Spanish, Asturian, etc. The model will learn that other languages have no punctuation before tokens and simply predict null.
- Implication 2: The model should never use a subword mask, to allow continuous-script languages to be punctuated and segmented between subword units. The model will learn that non-continuous languages are never broken or punctuated between subword units.
- Premise 1: Each analytic should be a function of the output of all dependent analytics.
- Implication 1: At inference time, the analytics are chained together such that punctuation sees unstructured text, capitalization sees punctuated text, and segmentation sees capitalized and punctuated text.
- Implication 2: At training time, the data loader produces 3 different versions of the inputs to provide reference data w.r.t. the particular analytic. Essentially, we are using teacher forcing to provide the reference output of dependent analytics.

The model's components are shown here, along with the training algorithm:

At inference time, it would run like this:

Beta Was this translation helpful? Give feedback.

All reactions

0 replies

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

### ekmb Mar 9, 2022 Collaborator

-

Hi @1-800-BAD-CODE, thank you for bringing up these valid issues and your willingness to contribute. It would be great if you could add a model that doesn't rely on subword masking with the punctuation head that predicts pre-and post- punctuation marks. I agree that having a separate model would make more sense, and we can then re-factor/depreciate the current model.

A few questions/comments:

1. If a training example for the punctuation model represents 1+ sentences, the model learns to split them into separate sentences, i.e., put the end of sentence punctuation marks in between the sentences. Paragraph segmentation seems redundant. Please also see this method that removes 'margin' probabilities when combining punctuation predictions from multiple segments (This was specifically added to segment long input texts for NMT task.)
2. I agree that capitalization should depend on punctuation. As for char-based tokenization, using a single LM would be preferable. The vocabularies of the most pre-trained transformer models include single chars (see BERT vocab.txt). However, the chars don't carry any semantic meaning, and the input sequence is getting longer (this will affect the segmentation head if you still want it). An alternative solution could be to use the same tokenization for the subword task, add punctuation head output, similarly to [this] (https://assets.amazon.science/55/3f/8c27b4014bdd983087fdb1d73412/robust-prediction-of-punctuation-and-truecasing-for-medical-asr.pdf) and use capitalization tags: {first_letter_upper, all_caps, all_lower, mixed (for
   ```
   McDonald
   ```
   cases)}.

Beta Was this translation helpful? Give feedback.

All reactions

1 reply

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### 1-800-BAD-CODE Mar 11, 2022 Author

-

Thanks for the feedback. I will implement something that uses only one LM and solves all problems in my original post, so not exactly like my initial plan outlined above.

A couple notes (anyone's feedback would be useful, but not necessary):

1. Based on my understanding of that paper (seemingly the most-cited deep-learning approach to this problem), they simply predicted whether a word was "mixed case" or not, without predicting *which* characters should be upper/lower case. My initial thought to solve this problem, utilizing a subword model, is to have the capitalization head makes *N* binary decisions per token, where *N* is the maximum subword length (default 16 for sentencepiece). Prediction *i* is whether subword letter *i* is capitalized. Whether this is feasible might best be answered experimentally.
2. Based on my understanding of that paper, capitalization for token
   ```
   i
   ```
   is conditioned only the predicted punctuation of token
   ```
   i
   ```
   . In practice, capitalization is usually dependent on token
   ```
   i
   ```
   ("Dr.") or
   ```
   i-1
   ```
   (new sentence). But I would say that the only sound conditioning method is to allow the LM to encode the newly-punctuated sequence, although at inference that requires multiple, sequential passes of the LM per query.

Beta Was this translation helpful? Give feedback.

All reactions

-... ### Uh oh!

  There was an error while loading. Please reload this page.
-

### ekmb May 19, 2022 Collaborator

-

Hi @1-800-BAD-CODE, any updates?

Beta Was this translation helpful? Give feedback.

All reactions

1 reply

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### 1-800-BAD-CODE May 30, 2022 Author

-

It's getting there. It's been unexpectedly-difficult to tune the punctuation loss weights, which is exacerbated by allowing predictions between subword tokens and predicting twice per token, greatly increasing the proportion of null class predictions.

I hope to have a more meaningful update with a couple of weeks.

Beta Was this translation helpful? Give feedback.

All reactions

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

### itzsimpl Jun 27, 2022

-

Hi @1-800-BAD-CODE, any updates? Very interested in this feature.

Beta Was this translation helpful? Give feedback.

All reactions

1 reply

- ### Uh oh!

  There was an error while loading. Please reload this page.
-

#### 1-800-BAD-CODE Jun 29, 2022 Author

-

It's still a work in progress, but active.

Beta Was this translation helpful? Give feedback.

All reactions

-

### 4. Punctuation and Capitalization Model - NVIDIA Docs Hub

**URL:** https://docs.nvidia.com/deeplearning/nemo/archives/nemo-100rc1/user-guide/docs/nlp/punctuation_and_capitalization.html

# Punctuation and Capitalization Model¶

Automatic Speech Recognition (ASR) systems typically generate text with no punctuation and capitalization of the words. There are two issues with non-punctuated ASR output:

it could be difficult to read and understand;

models for some downstream tasks such as named entity recognition, machine translation or text-to-speech are usually trained on punctuated datasets and using raw ASR output as the input to these models could deteriorate their performance.

## Quick Start¶

```

from nemo.collections.nlp.models import PunctuationCapitalizationModel

# to get the list of pre-trained models

PunctuationCapitalizationModel.list_available_models()

# Download and load the pre-trained BERT-based model

model = PunctuationCapitalizationModel.from_pretrained("punctuation_en_bert")

# try the model on a few examples

model.add_punctuation_capitalization(['how are you', 'great how about you'])

```

## Model Description¶

For each word in the input text, the Punctuation and Capitalization model:

predicts a punctuation mark that should follow the word (if any). By default, the model supports commas, periods and question marks.

predicts if the word should be capitalized or not.

In the Punctuation and Capitalization Model, we are jointly training two token-level classifiers on top of a pre-trained language model, such as [].

Note

We recommend you try this model in a Jupyter notebook (can run on .): .

Connect to an instance with a GPU (Runtime -> Change runtime type -> select “GPU” for hardware accelerator)

An example script on how to train the model could be found here: .

An example script on how to run evaluation and inference could be found here: .

The default configuration file for the model could be found at: .... ## Raw Data Format¶

The Punctuation and Capitalization model can work with any text dataset, although it is recommended to balance the data, especially for the punctuation task.

Before pre-processing the data to the format expected by the model, the data should be split into train.txt and dev.txt (and optionally test.txt).

Each line in the

**train.txt/dev.txt/test.txt** should represent one or more full and/or truncated sentences.

Example of the train.txt/dev.txt file:

```

When is the next flight to New York?

The next flight is ...

....

```

The source_data_dir structure should look like this:

```



|--sourced_data_dir
|-- dev.txt
|-- train.txt
```

## NeMo Data Format¶

The punctuation and capitalization model expects the data in the following format:

The training and evaluation data is divided into 2 files: text.txt and labels.txt. Each line of the

**text.txt** file contains text sequences, where words are separated with spaces, i.e.

[WORD] [SPACE] [WORD] [SPACE] [WORD], for example:

when is the next flight to new york the next flight is ... ...

The

**labels.txt** file contains corresponding labels for each word in text.txt, the labels are separated with spaces. Each label in labels.txt file consists of 2 symbols:

the first symbol of the label indicates what punctuation mark should follow the word (where O means no punctuation needed);

the second symbol determines if a word needs to be capitalized or not (where U indicates that the word should be upper cased, and O - no capitalization needed.)

By default the following punctuation marks are considered: commas, periods, and question marks; the rest punctuation marks were removed from the data. This can be changed by introducing new labels in the labels.txt files

Each line of the labels.txt should follow the format: [LABEL] [SPACE] [LABEL] [SPACE] [LABEL] (for labels.txt). For example, labels for the above text.txt file should be:

OU OO OO OO OO OO OU ?U OU OO OO OO ... ...

The complete list of all possible labels for this task used in this tutorial is: OO, ,O, .O, ?O, OU, ,U, .U, ?U.... ## Converting Raw Data to NeMo Format¶

To pre-process the raw text data, stored under

`sourced_data_dir` (see the

section), run the following command:

```

python examples/nlp/token_classification/data/prepare_data_for_punctuation_capitalization.py \

-s <PATH_TO_THE_SOURCE_FILE>

-o <PATH_TO_THE_OUTPUT_DIRECTORY>

```

### Required Argument for Dataset Conversion¶

`-s`or

`--source_file`: path to the raw file

`-o`or

`--output_dir`- path to the directory to store the converted files

After the conversion, the

`output_dir` should contain

`labels_*.txt` and

`text_*.txt` files.

The default names for the training and evaluation in the

`conf/punctuation_capitalization_config.yaml` are the following:

```



|--output_dir
|-- labels_dev.txt
|-- labels_train.txt
|-- text_dev.txt
|-- text_train.txt
```... ## Training Punctuation and Capitalization Model¶

The language model is initialized with the pre-trained model from , unless the user provides a pre-trained checkpoint for the language model, t Example of model configuration file for training the model could be found at: .

The specification can be roughly grouped into the following categories:

Parameters that describe the training process:

**trainer**

Parameters that describe the datasets:

**model.dataset**, **model.train_ds**, **model.validation_ds**

Parameters that describe the model:

**model**

More details about parameters in the config file could be found below and in the :

||||
|--|--|--|
|pretrained_model|string|Path to the pre-trained model .nemo file or pre-trained model name|
|model.dataset.data_dir|string|Path to the data converted to the specified above format|
|model.punct_head.punct_num_fc_layers|integer|Number of fully connected layers|
|model.punct_head.fc_dropout|float|Activation to use between fully connected layers|
|model.punct_head.activation|string|Dropout to apply to the input hidden states|
|model.punct_head.use_transrormer_init|bool|Whether to initialize the weights of the classifier head with the same approach used in Transformer|
|model.capit_head.punct_num_fc_layers|integer|Number of fully connected layers|
|model.capit_head.fc_dropout|float|Dropout to apply to the input hidden states|
|model.capit_head.activation|string|Activation function to use between fully connected layers|
|model.capit_head.use_transrormer_init|bool|Whether to initialize the weights of the classifier head with the same approach used in Transformer|
|training_ds.text_file|string|Name of the text training file located at data_dir|
|training_ds.labels_file|string|Name of the labels training file located at data_dir, such as labels_train.txt|
|training_ds.num_samples|integer|Number of samples to use from the training dataset, -1 - to use all|
|validation_ds.text_file|string|Name of the text file for evaluation, located at data_dir|
|validation_ds.labels_file|string|Name of the labels dev file located at data_dir, such as labels_dev.txt|
|validation_ds.num_samples|integer|Number of samples to use from the dev set, -1 mean all|
See also .... ## Inference¶

An example script on how to run inference on a few examples, could be found at .

To run inference with the pre-trained model on a few examples, run:

```

python punctuation_capitalization_evaluate.py \

pretrained_model=<PRETRAINED_MODEL>

```... ## Model Evaluation¶

An example script on how to evaluate the pre-trained model, could be found at .

To run evaluation of the pre-trained model, run:

```

python punctuation_capitalization_evaluate.py \

model.dataset.data_dir=<PATH/TO/DATA/DIR> \

pretrained_model=punctuation_en_bert \

model.test_ds.text_file=<text_dev.txt> \

model.test_ds.labels_file=<labels_dev.txt>

```

### Required Arguments¶

`pretrained_model`: pretrained PunctuationCapitalization model from list_available_models() or path to a .nemo file, for example: punctuation_en_bert or your_model.nemo

`model.dataset.data_dir`: Path to the directory that containes

`model.test_ds.text_file`and

`model.test_ds.labels_file`.

During evaluation of the

`test_ds`, the script generates two classification reports: one for capitalization task and another one for punctuation task. This classification reports include the following metrics:

`Precision`

`Recall`

`F1`

More details about these metrics could be found .

## References¶

- NLP-PUNCT1

Jacob Devlin, Ming-Wei Chang, Kenton Lee, and Kristina Toutanova. Bert: pre-training of deep bidirectional transformers for language understanding.

*arXiv preprint arXiv:1810.04805*, 2018.

### 5. nvidia/parakeet-tdt_ctc-1.1b - Hugging Face

**URL:** https://huggingface.co/nvidia/parakeet-tdt_ctc-1.1b

# Parakeet TDT-CTC 1.1B PnC(en)

`parakeet-tdt_ctc-1.1b` is an ASR model that transcribes speech with Punctuations and Capitalizations of English alphabet. This model is jointly developed by NVIDIA NeMo and Suno.ai teams.

It is an XXL version of Hybrid FastConformer [1] TDT-CTC [2] (around 1.1B parameters) model. This model has been trained with Local Attention and Global token hence this model can transcribe

**11 hrs** of audio in one single pass. And for reference this model can transcibe 90mins of audio in <16 sec on A100.

See the model architecture section and NeMo documentation for complete architecture details.

## NVIDIA NeMo: Training

To train, fine-tune or play with the model you will need to install NVIDIA NeMo. We recommend you install it after you've installed latest PyTorch version.

```

pip install nemo_toolkit['all']

```... ## How to Use this Model

The model is available for use in the NeMo toolkit [3], and can be used as a pre-trained checkpoint for inference or for fine-tuning on another dataset.

### Automatically instantiate the model

```

import nemo.collections.asr as nemo_asr

asr_model = nemo_asr.models.ASRModel.from_pretrained(model_name="nvidia/parakeet-tdt_ctc-1.1b")

```

### Transcribing using Python

First, let's get a sample

```

wget https://dldata-public.s3.us-east-2.amazonaws.com/2086-149220-0033.wav

```

Then simply do:

```

output = asr_model.transcribe(['2086-149220-0033.wav'])

print(output[0].text)

```

### Transcribing many audio files

By default model uses TDT to transcribe the audio files, to switch decoder to use CTC, use decoding_type='ctc'

```

python [NEMO_GIT_FOLDER]/examples/asr/transcribe_speech.py

pretrained_name="nvidia/parakeet-tdt_ctc-1.1b"

audio_dir="<DIRECTORY CONTAINING AUDIO FILES>"

```

### Input

This model accepts 16000 Hz mono-channel audio (wav files) as input.

### Output

This model provides transcribed speech as a string for a given audio sample.... ## Model Architecture

This model uses a Hybrid FastConformer-TDT-CTC architecture. FastConformer [1] is an optimized version of the Conformer model with 8x depthwise-separable convolutional downsampling. You may find more information on the details of FastConformer here: Fast-Conformer Model.

## Training

The NeMo toolkit [3] was used for finetuning this model for 20,000 steps over

`parakeet-tdt-1.1` model. This model is trained with this example script and this base config.

The tokenizers for these models were built using the text transcripts of the train set with this script.

### Datasets

The model was trained on 36K hours of English speech collected and prepared by NVIDIA NeMo and Suno teams.

The training dataset consists of private subset with 27K hours of English speech plus 9k hours from the following public PnC datasets:

- Librispeech 960 hours of English speech

- Fisher Corpus

- National Speech Corpus Part 1

- VCTK

- VoxPopuli (EN)

- Europarl-ASR (EN)

- Multilingual Librispeech (MLS EN) - 2,000 hour subset

- Mozilla Common Voice (v7.0)... ## Performance

The performance of Automatic Speech Recognition models is measuring using Word Error Rate. Since this dataset is trained on multiple domains and a much larger corpus, it will generally perform better at transcribing audio in general.

The following tables summarizes the performance of the available models in this collection with the Transducer decoder. Performances of the ASR models are reported in terms of Word Error Rate (WER%) with greedy decoding.

|Version|Tokenizer|Vocabulary Size|AMI|Earnings-22|Giga Speech|LS test-clean|LS test-other|SPGI Speech|TEDLIUM-v3|Vox Populi|Common Voice|
|--|--|--|--|--|--|--|--|--|--|--|--|
|1.23.0|SentencePiece Unigram|1024|15.94|11.86|10.19|1.82|3.67|2.24|3.87|6.19|8.69|
These are greedy WER numbers without external LM. More details on evaluation can be found at HuggingFace ASR Leaderboard

## Model Fairness Evaluation

As outlined in the paper "Towards Measuring Fairness in AI: the Casual Conversations Dataset", we assessed the parakeet-tdt_ctc-1.1b model for fairness. The model was evaluated on the CausalConversations-v1 dataset, and the results are reported as follows:

### Gender Bias:

|Gender|Male|Female|N/A|Other|
|--|--|--|--|--|
|Num utterances|19325|24532|926|33|
|% WER|12.81|10.49|13.88|23.12|
### Age Bias:

|Age Group|(18-30)|(31-45)|(46-85)|(1-100)|
|--|--|--|--|--|
|Num utterances|15956|14585|13349|43890|
|% WER|11.50|11.63|11.38|11.51|
(Error rates for fairness evaluation are determined by normalizing both the reference and predicted text, similar to the methods used in the evaluations found at https://github.com/huggingface/open_asr_leaderboard.)... ## NVIDIA Riva: Deployment

NVIDIA Riva, is an accelerated speech AI SDK deployable on-prem, in all clouds, multi-cloud, hybrid, on edge, and embedded. Additionally, Riva provides:

- World-class out-of-the-box accuracy for the most common languages with model checkpoints trained on proprietary data with hundreds of thousands of GPU-compute hours

- Best in class accuracy with run-time word boosting (e.g., brand and product names) and customization of acoustic model, language model, and inverse text normalization

- Streaming speech recognition, Kubernetes compatible scaling, and enterprise-grade support

Although this model isn’t supported yet by Riva, the list of supported models is here.

Check out Riva live demo.

## References

[1] Fast Conformer with Linearly Scalable Attention for Efficient Speech Recognition

[2] Efficient Sequence Transduction by Jointly Predicting Tokens and Durations

[3] Google Sentencepiece Tokenizer

[5] Suno.ai

[6] HuggingFace ASR Leaderboard

[7] Towards Measuring Fairness in AI: the Casual Conversations Dataset

## Licence

License to use this model is covered by the CC-BY-4.0. By downloading the public and release version of the model, you accept the terms and conditions of the CC-BY-4.0 license.

- Downloads last month

- 26,698... ## Model tree for nvidia/parakeet-tdt_ctc-1.1b

## Datasets used to train nvidia/parakeet-tdt_ctc-1.1b

## Space using nvidia/parakeet-tdt_ctc-1.1b 1

## Collection including nvidia/parakeet-tdt_ctc-1.1b

## Evaluation results

- Test WER on AMI (Meetings test)test set self-reported15.940

- Test WER on Earnings-22test set self-reported11.860

- Test WER on GigaSpeechtest set self-reported10.190

- Test WER on LibriSpeech (clean)test set self-reported1.820

- Test WER on LibriSpeech (other)test set self-reported3.670

- Test WER on SPGI Speechtest set self-reported2.240

- Test WER on tedlium-v3test set self-reported3.870

- Test WER on Vox Populitest set self-reported6.190

- Test WER on Mozilla Common Voice 9.0test set self-reported8.690

## Metadata

```json
{
  "planId": "plan_1",
  "executionTime": 67757,
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
