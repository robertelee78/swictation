"""
Monkey-patches for NeMo library bugs.

This module patches known issues in the NeMo library to ensure Swictation works correctly.
"""

import sys
from typing import List


def patch_aggregate_tokenizer_lang_id():
    """
    Patch NeMo's AggregateTokenizer to handle missing lang_id parameter.

    Bug: NeMo's chunking_utils.py calls decode_tokens_to_str() without lang parameter,
    which causes AggregateTokenizer.tokens_to_text() to fail with:
    "TypeError: AggregateTokenizer.tokens_to_text() missing 1 required positional argument: 'lang_id'"

    Solution: Wrap decode_tokens_to_str to always pass default lang='en' for Canary model.

    References:
    - /home/robert/.local/lib/python3.12/site-packages/nemo/collections/asr/parts/utils/chunking_utils.py:65
    - /home/robert/.local/lib/python3.12/site-packages/nemo/collections/asr/parts/submodules/multitask_decoding.py:534
    """
    try:
        from nemo.collections.asr.parts.submodules.multitask_decoding import MultiTaskDecoding

        # Save original method
        original_decode_tokens_to_str = MultiTaskDecoding.decode_tokens_to_str

        def patched_decode_tokens_to_str(self, tokens: List[str], lang: str = None) -> str:
            """
            Patched version that defaults to 'en' if lang is None and using AggregateTokenizer.
            """
            # Check if using AggregateTokenizer (has lang_id parameter)
            tokenizer_class_name = self.tokenizer.__class__.__name__

            if tokenizer_class_name == 'AggregateTokenizer' and lang is None:
                # Default to English for Canary multilingual model
                lang = 'en'

            # Call original method with lang parameter
            return original_decode_tokens_to_str(self, tokens, lang=lang)

        # Apply patch
        MultiTaskDecoding.decode_tokens_to_str = patched_decode_tokens_to_str

        print("✅ Applied NeMo patch: AggregateTokenizer lang_id fix", flush=True)
        return True

    except ImportError as e:
        print(f"⚠️  Could not apply NeMo patch (import failed): {e}", flush=True, file=sys.stderr)
        return False
    except Exception as e:
        print(f"❌ Failed to apply NeMo patch: {e}", flush=True, file=sys.stderr)
        return False


def apply_all_patches():
    """
    Apply all NeMo patches required for Swictation.

    Call this once at startup before loading NeMo models.

    Returns:
        int: Number of patches successfully applied
    """
    patches_applied = 0

    print("🔧 Applying NeMo compatibility patches...", flush=True)

    # Patch 1: AggregateTokenizer lang_id
    if patch_aggregate_tokenizer_lang_id():
        patches_applied += 1

    print(f"✓ Applied {patches_applied} NeMo patch(es)", flush=True)

    return patches_applied
