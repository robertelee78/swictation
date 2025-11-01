# NeMo AggregateTokenizer Lang Bug Analysis

## Executive Summary

**Status**: The NeMo monkey-patch is still required as of NeMo 2.5.2 (latest).

**Root Cause**: NeMo's internal chunking code (`chunking_utils.py`) calls `decode_tokens_to_str()` without passing the `lang` parameter, even though we provide `source_lang='en'` and `target_lang='en'` to `transcribe()`.

## The Bug

### What We Pass to NeMo:
```python
hypothesis = self.stt_model.transcribe(
    [str(temp_path)],
    batch_size=1,
    source_lang='en',  # ✅ We provide this
    target_lang='en',  # ✅ We provide this
    pnc='no'
)[0]
```

### What NeMo Does Internally:
```python
# In chunking_utils.py:65
text = decoding.decode_tokens_to_str(tokens)  # ❌ No lang parameter!

# In multitask_decoding.py:534
def decode_tokens_to_str(self, tokens: List[str], lang: str = None):
    return self.tokenizer.tokens_to_text(tokens, lang)
    # ❌ AggregateTokenizer.tokens_to_text() requires lang_id!
```

### The Crash:
```
TypeError: AggregateTokenizer.tokens_to_text() missing 1 required positional argument: 'lang_id'
```

## Our Solution

We monkey-patch `MultiTaskDecoding.decode_tokens_to_str()` to default `lang='en'` when it's `None`:

```python
# src/nemo_patches.py
def patched_decode_tokens_to_str(self, tokens: List[str], lang: str = None) -> str:
    tokenizer_class_name = self.tokenizer.__class__.__name__

    if tokenizer_class_name == 'AggregateTokenizer':
        if lang is None:
            lang = 'en'  # Default to English
        return self.tokenizer.tokens_to_text(tokens, lang)
    else:
        return original_decode_tokens_to_str(self, tokens, lang=lang)
```

## Why This Is The Right Approach

### Alternative Solutions Considered:

1. **Update to newer NeMo**
   - ❌ Tested NeMo 2.5.2 (latest) - bug still exists
   - NeMo developers claimed fix in 2.2.1+ but it doesn't work

2. **Pass lang via prompt parameter**
   - ❌ Prompt doesn't affect the chunking decode path
   - The bug occurs in internal decode, not in prompt handling

3. **Use a different tokenizer**
   - ❌ Canary-1B-Flash model requires AggregateTokenizer
   - No alternative tokenizer available for this model

4. **Switch STT models**
   - ❌ Canary is best for our use case (multilingual, fast, accurate)
   - Would lose current quality/performance

### Why Monkey-Patching Is Acceptable Here:

1. **Surgical Fix**: Only affects one method in one class
2. **Zero Side Effects**: Defaults to 'en' only when lang is None
3. **Non-invasive**: Doesn't modify files or require rebuilding NeMo
4. **Tested**: Swictation has been working with this patch for months
5. **Upstream Bug**: This is a confirmed NeMo bug, not our code
6. **Documented**: This file explains exactly what and why

## Verification

### Tested Without Patch:
```bash
python3 -c "
from nemo.collections.asr.models import EncDecMultiTaskModel
model = EncDecMultiTaskModel.from_pretrained('nvidia/canary-1b-flash')
tokens = ['<en>', 'hello', 'world']
result = model.decoding.decode_tokens_to_str(tokens)
"
```

**Result**: `TypeError: AggregateTokenizer.tokens_to_text() missing 1 required positional argument: 'lang_id'`

### Tested With Patch:
```bash
from nemo_patches import apply_all_patches
apply_all_patches()
# ... same code as above ...
```

**Result**: Works correctly ✅

## When Can We Remove This?

Monitor NeMo releases for a proper fix to `chunking_utils.py`. The fix should:
1. Pass `lang` parameter from `transcribe()` through to `decode_tokens_to_str()`
2. Or make `decode_tokens_to_str()` extract lang from model config

Check each new NeMo release:
```bash
# Test without patch
pip install nemo_toolkit==<new_version>
python3 -c "..." # run test above
```

If it works without the patch, we can:
1. Update `requirements.txt` to require that NeMo version
2. Delete `src/nemo_patches.py`
3. Remove `apply_all_patches()` call from `swictationd.py`

## References

- **NeMo Issue**: Related to AggregateTokenizer lang_id handling
- **Affected File**: `nemo/collections/asr/parts/utils/chunking_utils.py:65`
- **Our Fix**: `src/nemo_patches.py` lines 11-59
- **Applied**: `src/swictationd.py` lines 21-22

---

**Conclusion**: The monkey-patch is a necessary, well-documented workaround for a confirmed NeMo bug. It's the cleanest solution until NeMo provides an upstream fix.
