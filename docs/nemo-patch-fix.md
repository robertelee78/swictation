# NeMo AggregateTokenizer lang_id Bug Fix

## Problem

When using NVIDIA NeMo's Canary multilingual model (`nvidia/canary-1b-flash`), the daemon would crash with:

```python
TypeError: AggregateTokenizer.tokens_to_text() missing 1 required positional argument: 'lang_id'
```

### Root Cause

NeMo library bug in two locations:

1. **`chunking_utils.py:65`** - Calls `decode_tokens_to_str()` WITHOUT lang parameter
2. **`multitask_decoding.py:534`** - When `lang=None`, calls `tokenizer.tokens_to_text(tokens)` without `lang_id`

For the `AggregateTokenizer` (used in multilingual Canary models), the `tokens_to_text()` method **requires** a `lang_id` argument:

```python
# NeMo's code (simplified)
def decode_tokens_to_str(self, tokens, lang=None):
    if lang is not None:
        return self.tokenizer.tokens_to_text(tokens, lang)  # ✅ Works
    else:
        return self.tokenizer.tokens_to_text(tokens)        # ❌ Breaks for AggregateTokenizer!
```

### Error Traceback

```
File "/home/robert/.local/lib/python3.12/site-packages/nemo/collections/asr/parts/utils/chunking_utils.py", line 65, in merge_parallel_chunks
  final_text = decoding.decode_tokens_to_str(merged_tokens)  # No lang argument!
File "/home/robert/.local/lib/python3.12/site-packages/nemo/collections/asr/parts/submodules/multitask_decoding.py", line 534, in decode_tokens_to_str
  hypothesis = self.tokenizer.tokens_to_text(tokens)          # Missing lang_id!
TypeError: AggregateTokenizer.tokens_to_text() missing 1 required positional argument: 'lang_id'
```

## Solution: Monkey Patch

We created `/opt/swictation/src/nemo_patches.py` that monkey-patches NeMo's `MultiTaskDecoding.decode_tokens_to_str()` method **before** NeMo models are loaded.

### How It Works

```python
# In nemo_patches.py
def patched_decode_tokens_to_str(self, tokens, lang=None):
    tokenizer_class_name = self.tokenizer.__class__.__name__

    # Detect AggregateTokenizer and provide default lang
    if tokenizer_class_name == 'AggregateTokenizer' and lang is None:
        lang = 'en'  # Default to English

    # Call original method with lang parameter
    return original_decode_tokens_to_str(self, tokens, lang=lang)
```

### Integration

In `swictationd.py`, patches are applied **before** importing NeMo:

```python
# Apply NeMo patches BEFORE importing NeMo
from nemo_patches import apply_all_patches
apply_all_patches()

# Now safe to import NeMo
from nemo.collections.asr.models import EncDecMultiTaskModel
```

## Verification

Daemon startup logs now show:

```
🔧 Applying NeMo compatibility patches...
✅ Applied NeMo patch: AggregateTokenizer lang_id fix
✓ Applied 1 NeMo patch(es)
✓ Swictation daemon started
```

## Why This Works

1. **User speaks only English** - Defaulting to `lang='en'` is correct
2. **Canary is multilingual** - But we're using it for English-only dictation
3. **Monkey patch is safe** - Only affects `AggregateTokenizer`, doesn't break other tokenizers
4. **Applied early** - Runs before NeMo models load, ensuring patch is active

## Future Considerations

### Multi-Language Support

If we add multi-language support in the future, we can:

1. Detect language from user configuration
2. Pass detected language through the entire pipeline
3. Update patch to use configured language instead of hardcoded 'en'

Example:

```python
# Future enhancement
class SwictationDaemon:
    def __init__(self, target_lang='en', ...):
        self.target_lang = target_lang

        # Configure NeMo to use target language
        self.stt_model.set_target_lang(self.target_lang)
```

### Reporting to NeMo Team

This bug should be reported to NVIDIA NeMo:

- Repository: https://github.com/NVIDIA/NeMo
- Issue: `chunking_utils.py` should pass `lang` parameter to `decode_tokens_to_str()`
- Proposed fix: Add `lang` parameter extraction from model config or metadata

## Alternative Solutions (Not Used)

We considered these approaches but chose monkey-patching:

1. **Downgrade NeMo** ❌ - Loses bug fixes and performance improvements
2. **Fork NeMo** ❌ - Maintenance burden, difficult to update
3. **Disable chunk merging** ❌ - Loses accuracy for long audio
4. **Switch to different model** ❌ - Canary-1B is optimal for our use case
5. **Monkey patch** ✅ - Minimal code, easy to remove when NeMo fixes upstream

## Files Modified

- `/opt/swictation/src/nemo_patches.py` - New patch module (78 lines)
- `/opt/swictation/src/swictationd.py` - Added patch import (4 lines)

## Commit

```
fix: Add NeMo AggregateTokenizer lang_id monkey patch

Fixes TypeError when NeMo's chunking_utils.py calls decode_tokens_to_str()
without lang parameter for multilingual Canary model.
```

## References

- NeMo Issue: (To be filed)
- NeMo Docs: https://docs.nvidia.com/nemo-framework/
- Canary Model: https://huggingface.co/nvidia/canary-1b-flash
