# PRD: Text Transform v2 - Context-Aware Voice-to-Text for Claude Code

**Version**: 0.2.0
**Author**: Robert E. Lee
**Date**: 2025-11-21
**Status**: Draft

---

## Executive Summary

The current midstream text transformation rules were designed for "1950s secretary" style dictation. In theory, this works well for Claude Code usage where natural speech should mostly pass through unchanged, with explicit commands for symbol insertion. However, we have found instances where our existing Secretary Mode produces false positives that harm the Claude Code experience.

This PRD proposes an **additive three-layer architecture** that modifies the existing transform pipeline:
1. **Layer 1: Escape/Literal Detection** - Override mechanism to output words literally
2. **Layer 2: Explicit Phrase Matching** - Require trigger words for ambiguous conversions
3. **Layer 3: Mode-Aware Rules** - Modified Secretary Mode rules (current behavior adjusted)

**Note**: A future "Code Mode" for IDE-style editing is out of scope for this PRD.

---

## Problem Statement

### Current Issues

| Spoken Input | Current Output | Desired Output (Secretary Mode) |
|--------------|----------------|---------------------------|
| "Add one more test" | "Add 1 more test" | "Add one more test" |
| "There are two options" | "There are 2 options" | "There are two options" |
| "Hash the password" | "# the password" | "Hash the password" |
| "Doctor this code" | "Dr. this code" | "Doctor this code" |
| "Plus that feature" | "+ that feature" | "Plus that feature" |
| "In this period" | "In this ." | "In this period" |

### Root Cause

Rules trigger on single keywords without considering:
- Surrounding context (is "hash" a verb or symbol request?)
- Explicit user intent (was "number" spoken before digits?)

---

## Goals

1. **Preserve natural speech** - Common English words pass through unchanged
2. **Explicit symbol insertion** - Clear trigger phrases for intentional conversions
3. **Backward compatibility** - Keep "comma" → "," and "period" → "." (with literal escape)
4. **Extensibility** - Easy to add new operators with unambiguous triggers
5. **Performance** - Maintain <5ms transformation time for typical input

---

## Architecture

### Three-Layer Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                        INPUT TEXT                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 1: Escape/Literal Detection                              │
│  ─────────────────────────────────────────────────────────────  │
│  "literal comma" → "comma" (pass-through)                       │
│  "the word period" → "period" (pass-through)                    │
│  Processed FIRST to override all other layers                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 2: Explicit Phrase Matching                              │
│  ─────────────────────────────────────────────────────────────  │
│  "number forty two" → 42                                        │
│  "hash sign" → #                                                │
│  "pipe sign" → |                                                │
│  Multi-word patterns checked longest-first                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: Mode-Aware Rule Application                           │
│  ─────────────────────────────────────────────────────────────  │
│  Secretary Mode: Unambiguous rules only                         │
│  - "comma" → ","  (kept - use "literal comma" to escape)        │
│  - "period" → "." (kept - use "literal period" to escape)       │
│  - "one" → pass through (CHANGED - use "number one")            │
│  - "hash" → pass through (CHANGED - use "hash sign")            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       OUTPUT TEXT                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Layer 1: Escape/Literal Commands

### Purpose
Allow users to explicitly output words that would otherwise be converted.

### Trigger Phrases

| Phrase Pattern | Output | Example |
|----------------|--------|---------|
| "literal X" | X | "literal comma" → "comma" |
| "the word X" | X | "the word period" → "period" |
| "literally X" | X | "literally one" → "one" |
| "say X" | X | "say hash" → "hash" |

### Scope
- Applies to single words: "literal comma" → "comma"
- Applies to multi-word triggers: "literal open paren" → "open paren"
- Process BEFORE all other layers
- Remove trigger word, pass through the target word(s) unchanged

### Implementation Notes
- Check for escape triggers first in the transform loop
- When matched, skip Layer 2 and Layer 3 processing for escaped word(s)
- Preserve original casing of escaped word

---

## Layer 2: Explicit Phrase Matching

### Purpose
Require unambiguous multi-word phrases for conversions that would otherwise cause false positives.

### Number Conversions

**Core Rule Change**: Standalone number words ("one", "two", "forty two") now pass through unchanged. Use explicit triggers to convert.

| Phrase | Output | Notes |
|--------|--------|-------|
| "number X" | digit | "number one" → "1", "number forty two" → "42" |
| "digit X" | digit | Alternative trigger |
| "line X" | line + digit | "line forty two" → "line 42" (keeps "line") |
| "version X" | version + digit | "version two" → "version 2" |
| "step X" | step + digit | "step one" → "step 1" |
| "option X" | option + digit | "option three" → "option 3" |
| "error X" | error + digit | "error four oh four" → "error 404" |
| "port X" | port + digit | "port eighty eighty" → "port 8080" |
| "release X" | release + digit | "release zero point four" → "release 0.4" |

**Year Patterns** (existing behavior, kept):
- "number nineteen fifty" → "1950"
- "number twenty twenty five" → "2025"
- "number nineteen fifties" → "1950s"

### Symbol Conversions - Ambiguous Words Now Require Phrases

| Word | Old Behavior | New Behavior |
|------|--------------|--------------|
| hash | → # | Pass through (use "hash sign") |
| pound | → # | Pass through (use "pound sign") |
| plus | → + | Pass through (use "plus sign") |
| equals | → = | Pass through (use "equals sign") |
| one, two, etc. | → 1, 2, etc. | Pass through (use "number X") |
| doctor | → Dr. | Pass through (disabled) |
| mister | → Mr. | Pass through (disabled) |
| period | → . | **KEPT** (use "literal period" to escape) |
| comma | → , | **KEPT** (use "literal comma" to escape) |

### Explicit Symbol Triggers (Required for Ambiguous)

| Phrase | Output | Notes |
|--------|--------|-------|
| "hash sign" | # | Explicit |
| "hashtag" | # | Social media context (kept) |
| "pound sign" | # | Explicit |
| "plus sign" | + | Explicit |
| "minus sign" | - | Explicit |
| "equals sign" | = | Explicit |
| "pipe sign" | \| | Explicit (disambiguates from plumbing) |

### Unambiguous Symbols (Keep Current Behavior)

These single words have no common English meaning conflicts and remain as-is:

| Voice Command | Output | Notes |
|---------------|--------|-------|
| "comma" | , | Kept |
| "period" / "full stop" | . | Kept |
| "question mark" | ? | Kept |
| "exclamation point" | ! | Kept |
| "colon" | : | Kept |
| "semicolon" | ; | Kept |
| "at sign" | @ | Kept |
| "dollar sign" | $ | Kept |
| "percent sign" / "percent" | % | Kept |
| "ampersand" | & | Kept |
| "asterisk" | * | Kept |
| "forward slash" / "slash" | / | Kept |
| "backslash" | \ | Kept |
| "open paren" / "close paren" | ( ) | Kept |
| "open bracket" / "close bracket" | [ ] | Kept |
| "open brace" / "close brace" | { } | Kept |
| "quote" / "single quote" | " ' | Kept |
| "new line" / "new paragraph" | \n \n\n | Kept |

---

## New Rules to Add

### Programming Operators (Unambiguous - Single Word OK)

| Phrase | Output | Category |
|--------|--------|----------|
| "backtick" | ` | Unambiguous |
| "triple backtick" / "code fence" | ``` | Unambiguous |
| "underscore" | _ | Unambiguous |
| "tilde" | ~ | Unambiguous |
| "caret" / "carrot" | ^ | Unambiguous |
| "double equals" | == | Unambiguous |
| "triple equals" | === | Unambiguous |
| "not equals" / "bang equals" | != | Unambiguous |
| "strict not equals" | !== | Unambiguous |
| "double colon" | :: | Unambiguous |
| "double ampersand" / "and and" | && | Unambiguous |
| "double pipe" / "or or" | \|\| | Unambiguous |
| "less than or equal" | <= | Unambiguous |
| "greater than or equal" | >= | Unambiguous |
| "plus equals" | += | Unambiguous |
| "minus equals" | -= | Unambiguous |
| "times equals" | *= | Unambiguous |
| "divide equals" | /= | Unambiguous |
| "increment" | ++ | Unambiguous |
| "decrement" | -- | Unambiguous |
| "spread" / "splat" / "triple dot" | ... | Unambiguous |
| "null coalesce" | ?? | Unambiguous |
| "optional chain" | ?. | Unambiguous |
| "angle brackets" | <> | Unambiguous |

### Directional Operators (Require Modifier to Disambiguate)

| Phrase | Output | Notes |
|--------|--------|-------|
| "right arrow" / "thin arrow" | -> | Disambiguates from bow/arrow |
| "left arrow" | <- | Disambiguates from bow/arrow |
| "fat arrow" / "rocket" | => | Unambiguous |
| "up arrow" | ↑ | Explicit direction |
| "down arrow" | ↓ | Explicit direction |
| "less than" / "left angle" | < | Context-aware |
| "greater than" / "right angle" | > | Context-aware |

---

## Behavioral Changes Summary

### Words That Now Pass Through (Breaking Changes)

| Word | Old | New | To Get Symbol |
|------|-----|-----|---------------|
| one | 1 | one | "number one" |
| two | 2 | two | "number two" |
| ... (all number words) | digit | word | "number X" |
| hash | # | hash | "hash sign" |
| pound | # | pound | "pound sign" |
| plus | + | plus | "plus sign" |
| equals | = | equals | "equals sign" |
| doctor | Dr. | doctor | (disabled) |
| mister | Mr. | mister | (disabled) |
| pipe | \| | pipe | "pipe sign" |
| arrow | → | arrow | "right arrow" / "left arrow" |

### Words That Stay the Same

| Word | Output | Escape With |
|------|--------|-------------|
| comma | , | "literal comma" |
| period | . | "literal period" |
| question mark | ? | "literal question mark" |
| colon | : | "literal colon" |
| semicolon | ; | "literal semicolon" |
| slash | / | "literal slash" |
| backtick | ` | "literal backtick" |

---

## Example Transformations

### Before v2 (Current)
```
Input:  "Add one more test"
Output: "Add 1 more test"  ← WRONG

Input:  "Hash the password"
Output: "# the password"   ← WRONG

Input:  "Doctor this code"
Output: "Dr. this code"    ← WRONG
```

### After v2 (Proposed)
```
Input:  "Add one more test"
Output: "Add one more test"  ← CORRECT (natural speech)

Input:  "Add number one more test"
Output: "Add 1 more test"    ← CORRECT (explicit trigger)

Input:  "Hash the password"
Output: "Hash the password"  ← CORRECT (natural speech)

Input:  "Hash sign define"
Output: "# define"           ← CORRECT (explicit trigger)

Input:  "Doctor this code"
Output: "Doctor this code"   ← CORRECT (natural speech)

Input:  "In this period we saw growth"
Output: "In this . we saw growth"  ← Still converts (use "literal period")

Input:  "In this literal period we saw growth"
Output: "In this period we saw growth"  ← CORRECT (escaped)
```

---

## Implementation Plan

### Phase 1: Layer 1 - Escape/Literal Detection
1. Add escape trigger detection at start of transform loop
2. Implement "literal X", "the word X", "literally X", "say X" patterns
3. Support multi-word escapes: "literal open paren"
4. Add tests for all escape patterns

### Phase 2: Layer 2 - Explicit Phrase Matching Updates
1. **Remove** standalone number word mappings from STATIC_MAPPINGS
2. Keep "number X" prefix logic (already exists)
3. Add new contextual triggers: "line X", "version X", "step X", etc.
4. Add new symbol phrase mappings: "hash sign", "plus sign", etc.

### Phase 3: Layer 3 - Ambiguous Word Removal
1. Remove/disable these from STATIC_MAPPINGS:
   - hash, pound, plus, equals (require phrase)
   - doctor, mister, missus, ms, miss (disable completely)
   - one, two, three... ninety (require "number X")
2. Keep: comma, period, colon, semicolon, etc.

### Phase 4: New Operator Rules
1. Add programming operators (backtick, tilde, caret, etc.)
2. Add compound operators (double equals, not equals, etc.)
3. Add directional arrows with modifiers

### Phase 5: Testing & Documentation
1. Update all existing tests for new behavior
2. Add comprehensive tests for Layer 1 escapes
3. Add tests for explicit phrase triggers
4. Update docs/secretary-mode.md

---

## Migration & Compatibility

### User Impact
Users who relied on:
- "hash" → # must now say "hash sign"
- "one" → 1 must now say "number one"
- "doctor" → Dr. must now say... (disabled, no alternative)

### Recommended Communication
1. Changelog entry explaining behavioral changes
2. Update secretary-mode.md with new patterns
3. Consider "legacy mode" flag if rollback needed

---

## Performance Requirements

- Layer 1 (Escape): O(1) - Simple prefix check
- Layer 2 (Phrase): O(1) - HashMap lookup (existing)
- Layer 3 (Rules): O(1) - HashMap lookup (existing)
- **Total target**: <5ms for typical 10-word input (unchanged)

---

## Testing Strategy

### Unit Tests (Layer-specific)

```rust
// Layer 1: Escape tests
assert_eq!(transform("literal comma"), "comma");
assert_eq!(transform("the word period"), "period");
assert_eq!(transform("literally one"), "one");
assert_eq!(transform("literal open paren"), "open paren");

// Layer 2: Explicit phrase tests
assert_eq!(transform("hash sign"), "#");
assert_eq!(transform("number forty two"), "42");
assert_eq!(transform("line forty two"), "line 42");
assert_eq!(transform("version two"), "version 2");
assert_eq!(transform("pipe sign"), "|");

// Layer 3: Pass-through tests (ambiguous words)
assert_eq!(transform("hash the password"), "hash the password");
assert_eq!(transform("add one more"), "add one more");
assert_eq!(transform("plus that feature"), "plus that feature");
assert_eq!(transform("doctor this code"), "doctor this code");

// Layer 3: Kept behavior tests
assert_eq!(transform("hello comma world"), "hello, world");
assert_eq!(transform("stop period"), "stop.");
```

### Integration Tests

```rust
// Real-world Claude Code scenarios
assert_eq!(
    transform("please add number two more tests"),
    "please add 2 more tests"
);
assert_eq!(
    transform("there are two options"),
    "there are two options"
);
assert_eq!(
    transform("in the literal period between releases"),
    "in the period between releases"
);
```

---

## Open Questions (Resolved)

1. ~~**Sentence-ending period**: How to detect "end of thought" vs mid-sentence "period"?~~
   **Resolution**: Keep "period" → "." behavior. Users can escape with "literal period".

2. ~~**Number handling**: Should standalone numbers convert?~~
   **Resolution**: No. Require "number X" trigger for all number conversions.

3. ~~**Arrow disambiguation**: Should "arrow" alone convert?~~
   **Resolution**: No. Require directional modifier: "right arrow", "left arrow", etc.

---

## Appendix: Complete Rule Changes

### Rules to REMOVE from STATIC_MAPPINGS

```rust
// Number words (require "number X" trigger)
"zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
"ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
"seventeen", "eighteen", "nineteen", "twenty", "thirty", "forty", "fifty",
"sixty", "seventy", "eighty", "ninety", "hundred"

// Ambiguous symbols (require explicit phrase)
"hash", "pound", "plus", "equals"

// Titles (disabled - too many false positives)
"mister", "mr", "missus", "mrs", "doctor", "dr", "ms", "miss"

// Ambiguous operators
"pipe", "arrow"
```

### Rules to ADD to STATIC_MAPPINGS

```rust
// Explicit symbol phrases
"hash sign" → "#"
"pound sign" → "#"
"plus sign" → "+"
"minus sign" → "-"
"equals sign" → "="
"pipe sign" → "|"

// Directional arrows
"right arrow" → "->"
"left arrow" → "<-"
"up arrow" → "↑"
"down arrow" → "↓"
"fat arrow" → "=>"
"thin arrow" → "->"

// New programming operators
"backtick" → "`"
"triple backtick" → "```"
"code fence" → "```"
"underscore" → "_"
"tilde" → "~"
"caret" → "^"
"carrot" → "^"  // Common misspelling
"double equals" → "=="
"triple equals" → "==="
"not equals" → "!="
"bang equals" → "!="
"strict not equals" → "!=="
"double colon" → "::"
"double ampersand" → "&&"
"and and" → "&&"
"double pipe" → "||"
"or or" → "||"
"less than or equal" → "<="
"greater than or equal" → ">="
"plus equals" → "+="
"minus equals" → "-="
"times equals" → "*="
"divide equals" → "/="
"increment" → "++"
"decrement" → "--"
"spread" → "..."
"splat" → "..."
"triple dot" → "..."
"null coalesce" → "??"
"optional chain" → "?."
"angle brackets" → "<>"
```

### Escape Layer Triggers to ADD

```rust
// Layer 1: Escape patterns (process FIRST)
"literal {X}" → "{X}"      // "literal comma" → "comma"
"the word {X}" → "{X}"     // "the word period" → "period"
"literally {X}" → "{X}"    // "literally one" → "one"
"say {X}" → "{X}"          // "say hash" → "hash"
```
