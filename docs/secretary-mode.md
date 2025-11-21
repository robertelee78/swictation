# Secretary Dictation Mode

**Natural language dictation inspired by 1950s secretary stenography**

Secretary Mode is the first transformation mode in Swictation, designed for natural prose dictation where you speak punctuation, formatting, and special commands exactly as you would dictate to a secretary.

**Version:** 2.0.0 (November 2025) - Three-Layer Architecture

---

## 🎯 What is Secretary Mode?

Secretary Mode transforms spoken voice commands into their written equivalents in real-time:

- Say **"comma"** → Get **,**
- Say **"mr smith said quote hello quote exclamation point"** → Get **"Mr. Smith said 'Hello'!"**
- Say **"number forty two"** → Get **42**

This mode is perfect for:
- Writing emails, letters, and documents
- Dictating meeting notes
- Creating content naturally without keyboard
- Accessibility and hands-free writing

---

## 🏗️ Three-Layer Architecture (v2)

Secretary Mode v2 uses a three-layer processing pipeline:

### Layer 1: Escape/Literal Detection
Override mechanism to output words literally (processed FIRST):
```
"literal comma" → "comma"
"the word period" → "period"
"literally one" → "one"
```

### Layer 2: Explicit Phrase Matching
Require trigger words for ambiguous conversions:
```
"number forty two" → "42"
"hash sign" → "#"
"line forty two" → "line 42"
```

### Layer 3: Mode-Aware Rules
Modified Secretary Mode rules (adjusted for natural speech):
```
"comma" → ","  (kept - use "literal comma" to escape)
"one" → pass through (use "number one" for digit)
"hash" → pass through (use "hash sign" for symbol)
```

---

## 📋 Complete Feature Reference

### A. Basic Punctuation

All punctuation attaches to the previous word (no space before):

| Voice Command | Output | Example |
|--------------|--------|---------|
| comma | , | "Hello comma world" → "Hello, world" |
| period / full stop | . | "End period" → "End." |
| question mark | ? | "Why question mark" → "Why?" |
| exclamation point / exclamation mark | ! | "Stop exclamation point" → "Stop!" |
| colon | : | "Note colon" → "Note:" |
| semicolon | ; | "First semicolon second" → "First; second" |
| dash | - | "Well dash known" → "Well-known" |
| ellipsis / three dots | ... | "Wait ellipsis" → "Wait..." |

**Escape if needed:**
```
"literal comma" → "comma"
"the word period" → "period"
```

---

### B. Parentheses & Brackets

Opening brackets/parentheses preserve space before them. Closing ones attach to previous word:

| Voice Command | Output | Example |
|--------------|--------|---------|
| open paren / open parenthesis / open parentheses | ( | "Text open paren note close paren" → "Text (note)" |
| close paren / close parenthesis / close parentheses | ) | - |
| open bracket / open brackets | [ | "Array open bracket close bracket" → "Array []" |
| close bracket / close brackets | ] | - |
| open brace / open braces | { | "Object open brace close brace" → "Object {}" |
| close brace / close braces | } | - |

---

### C. Quotes (Stateful Toggle)

Quotes intelligently toggle between opening and closing. First word inside quotes is automatically capitalized:

| Voice Command | Output | Example |
|--------------|--------|---------|
| quote | " (toggles) | "She said quote hello quote" → "She said \"Hello\"" |
| open quote | " | - |
| close quote | " | - |
| single quote | ' (toggles) | "It quote s nice" → "It's nice" |
| apostrophe | ' (attach) | For contractions: "don apostrophe t" → "don't" |
| backtick | ` (toggles) | "backtick code backtick" → "`code`" |

---

### D. Special Symbols (v2: Ambiguous words require explicit phrases)

| Voice Command | Output | Example |
|--------------|--------|---------|
| dollar sign | $ | "dollar sign fifty" → "$ Fifty" |
| percent sign / percent | % | "fifty percent" → "Fifty %" |
| at sign | @ | "Email at sign example" → "Email @ example" |
| ampersand | & | "You ampersand me" → "You & me" |
| asterisk | * | "Note asterisk" → "Note *" |
| **hash sign** / **pound sign** | # | "hash sign tag" → "# Tag" |
| hashtag | # | "hashtag trending" → "# Trending" (social media) |
| forward slash / slash | / | "And slash or" → "And / or" |
| backslash | \ | "Path backslash file" → "Path \ file" |

**v2 Changes:**
- ❌ "hash" → passes through (use "hash sign")
- ❌ "pound" → passes through (use "pound sign")
- ✅ "hashtag" → "#" (unambiguous social media term)

---

### E. Math & Operators

| Voice Command | Output | Example |
|--------------|--------|---------|
| **plus sign** | + | "x plus sign y" → "x + y" |
| **minus sign** / minus | - | "a minus sign b" → "a - b" |
| **equals sign** | = | "x equals sign y" → "x = y" |
| equal sign | = | "a equal sign b" → "a = b" (alternative) |
| times / multiply | × | "two times three" → "two × three" |
| less than / left angle | < | "x less than y" → "x < y" |
| greater than / right angle | > | "x greater than y" → "x > y" |

**v2 Changes:**
- ❌ "plus" → passes through (use "plus sign")
- ❌ "equals" → passes through (use "equals sign")

---

### F. Programming Operators (v2: 40+ new operators)

#### Comparison & Logical
| Voice Command | Output | Example |
|--------------|--------|---------|
| double equals | == | "if x double equals y" → "if x == y" |
| triple equals | === | "strict check triple equals" → "strict check ===" |
| not equals / bang equals | != | "a not equals b" → "a != b" |
| strict not equals | !== | "a strict not equals b" → "a !== b" |
| less than or equal | <= | "x less than or equal five" → "x <= five" |
| greater than or equal | >= | "x greater than or equal ten" → "x >= ten" |
| double ampersand / and and | && | "a double ampersand b" → "a && b" |
| double pipe / or or | \|\| | "a double pipe b" → "a \|\| b" |

#### Special Programming Symbols
| Voice Command | Output | Example |
|--------------|--------|---------|
| underscore | _ | "snake underscore case" → "snake_case" |
| backtick | ` | "backtick code backtick" → "`code`" |
| triple backtick / code fence | ``` | "triple backtick python" → "```python" |
| tilde | ~ | "tilde home" → "~ home" |
| caret / carrot | ^ | "x caret y" → "x ^ y" |
| double colon | :: | "std double colon string" → "std :: string" |
| angle brackets | <> | "angle brackets" → "<>" |
| **pipe sign** | \| | "pipe sign input" → "\| input" |

**v2 Changes:**
- ❌ "pipe" → passes through (use "pipe sign")

#### Assignment Operators
| Voice Command | Output | Example |
|--------------|--------|---------|
| plus equals | += | "x plus equals one" → "x += one" |
| minus equals | -= | "x minus equals two" → "x -= two" |
| times equals | *= | "x times equals three" → "x *= three" |
| divide equals | /= | "x divide equals four" → "x /= four" |
| increment | ++ | "i increment" → "i ++" |
| decrement | -- | "i decrement" → "i --" |

#### Modern JavaScript/TypeScript
| Voice Command | Output | Example |
|--------------|--------|---------|
| spread / splat / triple dot | ... | "spread args" → "... args" |
| null coalesce | ?? | "x null coalesce default" → "x ?? default" |
| optional chain | ?. | "obj optional chain prop" → "obj ?. prop" |
| fat arrow / rocket | => | "fat arrow function" → "=> function" |
| thin arrow / right arrow | -> | "thin arrow ptr" → "-> ptr" |
| left arrow | <- | "left arrow back" → "<- back" |
| up arrow | ↑ | "up arrow" → "↑" |
| down arrow | ↓ | "down arrow" → "↓" |

---

### G. Formatting Commands

| Voice Command | Output | Example |
|--------------|--------|---------|
| new line | \n | "Line one new line line two" → "Line one\nline two" |
| new paragraph | \n\n | "Para one new paragraph para two" → "Para one\n\npara two" |
| tab | \t | "Indent tab text" → "Indent\ttext" |

---

### H. Abbreviations (v2: Titles removed)

| Voice Command | Output | Example |
|--------------|--------|---------|
| et cetera | etc. | "and so on et cetera" → "and so on etc." |
| versus | vs. | "team a versus team b" → "team a vs. team b" |
| post script | P.S. | "post script thanks" → "P.S. Thanks" |

**v2 Changes - Titles Removed:**
- ❌ "mister" / "mr" → passes through (ambiguous: "Mr. Smith" vs "mister president")
- ❌ "doctor" / "dr" → passes through (ambiguous: "Dr. Jones" vs "doctor this code")
- ❌ "missus" / "mrs" / "ms" / "miss" → passes through

**Workaround:** Use capitalization commands for proper nouns (see below).

---

### I. Number Conversion (v2: Explicit Triggers Required)

**Core Rule Change:** Standalone number words now pass through unchanged.

#### Primary Trigger: "number X" or "digit X"
| Voice Command | Output | Example |
|--------------|--------|---------|
| number zero | 0 | "number zero" → "0" |
| number five | 5 | "number five" → "5" |
| number forty two | 42 | "number forty two" → "42" |
| digit five | 5 | "digit five" → "5" (alternative) |
| digit forty two | 42 | "digit forty two" → "42" |

**Pass-through behavior:**
```
"one" → "one" (NOT "1")
"two options" → "two options" (NOT "2 options")
"add one more" → "add one more" (NOT "add 1 more")
```

#### Year Patterns
Years require "number" trigger but auto-detect teen+decade patterns:
```
"number nineteen fifty" → "1950"
"number twenty twenty five" → "2025"
"number nineteen ninety nine" → "1999"
"number nineteen fifties" → "1950s" (decade plurals)
```

#### Contextual Number Triggers
These keep the prefix word and convert the number:
| Voice Command | Output | Example |
|--------------|--------|---------|
| line X | line # | "line forty two" → "line 42" |
| version X | version # | "version two" → "version 2" |
| step X | step # | "step one" → "step 1" |
| option X | option # | "option three" → "option 3" |
| error X | error # | "error four oh four" → "error 404" |
| port X | port # | "port eighty eighty" → "port 8080" |
| release X | release # | "release twenty five" → "release 25" |

---

### J. Escape/Literal Commands (v2: Layer 1)

Force words to output literally (overrides all other transformations):

| Phrase Pattern | Output | Example |
|----------------|--------|---------|
| literal X | X | "literal comma" → "comma" |
| the word X | X | "the word period" → "period" |
| literally X | X | "literally one" → "one" |
| say X | X | "say hash" → "hash" |

**Multi-word support:**
```
"literal open paren" → "open paren"
"the word hash sign" → "hash sign"
```

---

### K. Capitalization Modes

#### Caps Mode (Toggle)
Turn on/off uppercase mode for multiple words:

```
"caps on hello world caps off" → "HELLO WORLD"
"normal caps on loud caps off quiet" → "normal LOUD quiet"
```

#### All Caps (Single Word)
Capitalize a single word (for acronyms):

```
"the all caps fbi investigated" → "the FBI investigated"
"all caps nasa launched" → "all caps NASA launched"
```

#### Capital Letter Command (Proper Nouns)
Explicitly capitalize specific words with **"capital [letter] [word]"**:

```
"capital r robert smith" → "Robert smith"
"my name is capital j jones" → "my name is Jones"
"capital n new capital y york" → "New York"
```

**Use case:** Since v2 removed title abbreviations, use this for proper nouns:
```
"capital m mister capital s smith" → "Mister Smith"
"capital d doctor capital j jones" → "Doctor Jones"
```

---

### L. Automatic Capitalization

Secretary Mode automatically capitalizes in these contexts:

#### 1. First-Person Pronoun "I"
```
"i am here" → "I am here"
"yes i am" → "yes I am"
"i'm happy" → "I'm happy"
"i'll go" → "I'll go"
```

#### 2. Sentence Starts
After period, exclamation point, or question mark:

```
"hello period world" → "hello. World"
"stop exclamation point go" → "stop! Go"
"why question mark because" → "why? Because"
```

#### 3. After Opening Quotes
First word inside quotes is capitalized:

```
"she said quote hello world quote" → "she said \"Hello world\""
"quote attention quote she yelled" → "\"Attention\" she yelled"
```

---

## 🚀 Usage Examples

### Example 1: Programming with v2 Features
**Voice:**
```
"function calculate underscore sum open paren nums close paren open brace
new line return nums period reduce open paren open paren a comma b close paren
fat arrow a plus sign b close paren semicolon new line close brace"
```

**Output:**
```
function calculate_sum(nums) {
return nums.reduce((a, b) => a + b);
}
```

### Example 2: Natural Speech (v2 Pass-through)
**Voice:**
```
"There are two options here period First comma we can add one more test period
Second comma we can hash the password for security period"
```

**Output:**
```
There are two options here. First, we can add one more test. Second, we can hash the password for security.
```

### Example 3: Explicit Number Conversion
**Voice:**
```
"We need number forty two tests comma not just two tests period
Error four oh four means not found period"
```

**Output:**
```
We need 42 tests, not just two tests. Error 404 means not found.
```

### Example 4: Escape Commands
**Voice:**
```
"Use literal comma to output the word comma period
Say literally one to get the text one instead of number one period"
```

**Output:**
```
Use comma to output the word comma. Say one to get the text one instead of 1.
```

---

## 🎯 Best Practices

### 1. Use Explicit Triggers for Symbols (v2)
Don't rely on ambiguous single words:
- ✅ "hash sign tag" → "# tag"
- ❌ "hash tag" → "hash tag" (passes through)
- ✅ "number five" → "5"
- ❌ "five" → "five" (passes through)

### 2. Escape When Needed
Use escape commands for literal text:
- ✅ "literal comma separated values" → "comma separated values"
- ✅ "the word period in time" → "period in time"

### 3. Contextual Number Triggers
Use contextual triggers to keep prefix words:
- ✅ "line forty two" → "line 42"
- ✅ "version two" → "version 2"
- ❌ "line number forty two" → "line 42" (redundant)

### 4. Year Numbers
Use the teen+decade pattern with "number" trigger:
```
"number nineteen fifty" → "1950" ✓
"number twenty twenty five" → "2025" ✓
```

---

## 🔧 Technical Details

### Architecture

Secretary Mode is implemented in the **MidStream text-transform** crate:
- **Rules:** `/external/midstream/crates/text-transform/src/rules.rs` (80+ static mappings)
- **Transform:** `/external/midstream/crates/text-transform/src/lib.rs` (three-layer pipeline, capitalization, state tracking)
- **Pipeline:** `/rust-crates/swictation-daemon/src/pipeline.rs` (integrates transformation after STT)

### Performance
- **Latency:** ~5µs per transformation (HashMap O(1) lookup)
- **Memory:** Static rules (zero allocation)
- **Target:** <5ms total transformation latency
- **Tests:** 66 passing tests (25 lib + 27 transform + 6 programming + 6 number + 2 doc)

### Three-Layer Processing Pipeline

```
[Microphone] → [VAD] → [STT] → [Transform] → [Text Injection]
                                      ↓
                        ┌─────────────────────────┐
                        │ Layer 1: Escape/Literal │
                        │ "literal comma" → skip  │
                        └───────────┬─────────────┘
                                    ↓
                        ┌─────────────────────────┐
                        │ Layer 2: Explicit Phrase│
                        │ "number X" → digit      │
                        │ "hash sign" → #         │
                        └───────────┬─────────────┘
                                    ↓
                        ┌─────────────────────────┐
                        │ Layer 3: Mode Rules     │
                        │ "comma" → ,             │
                        │ "one" → pass through    │
                        └─────────────────────────┘
```

1. **VAD** detects speech segments (0.8s silence threshold)
2. **STT** transcribes to lowercase text: "number forty two comma hash sign tag"
3. **Transform** applies three-layer rules:
   - Layer 1: Check for escape triggers
   - Layer 2: "number forty two" → "42", "hash sign" → "#"
   - Layer 3: "comma" → ","
4. **Result:** "42, # tag"
5. **Text Injection** types it into active window

### Spacing Between VAD Chunks

Secretary Mode automatically adds trailing spaces between VAD chunks:
```
Chunk 1: "hello world."  [0.8s silence]
Chunk 2: "testing"       [0.8s silence]
Result:  "hello world. testing"  ✓ (space added)
```

---

## 🚧 v2 Breaking Changes

### Words That Now Pass Through

| Word | v1 Behavior | v2 Behavior | To Get Symbol |
|------|-------------|-------------|---------------|
| one, two, ..., ninety | → digit | Pass through | "number one" |
| hash | → # | Pass through | "hash sign" |
| pound | → # | Pass through | "pound sign" |
| plus | → + | Pass through | "plus sign" |
| equals | → = | Pass through | "equals sign" |
| pipe | → \| | Pass through | "pipe sign" |
| doctor, mister, etc. | → Dr., Mr. | Pass through | Use "capital" commands |

### Words That Stay the Same

| Word | Output | Escape With |
|------|--------|-------------|
| comma | , | "literal comma" |
| period | . | "literal period" |
| question mark | ? | "literal question mark" |
| colon | : | "literal colon" |

---

## 🚧 Limitations & Future Work

### Current Limitations
- **No context awareness:** Can't distinguish "period" (punctuation) vs "period" (time interval) - use "literal period" to escape
- **Fixed capitalization rules:** May capitalize words you don't want (e.g., after abbreviations)
- **No speaker adaptation:** Doesn't learn your personal voice patterns (yet)

### Future Enhancements (Tier 2 & 3)
- **Tier 2: Adaptive Pattern Learning** (task 7e734c60)
  - Learn your personal variations and speaking style
  - Adapt to how YOU pronounce commands
  - Store user-specific patterns persistently

- **Tier 3: Intelligent Temporal Prediction** (task 50a6b24d)
  - Predict transformations based on dictation rhythm
  - Context-aware disambiguation
  - Meta-learning from your dictation history

---

## 🐛 Troubleshooting

### Issue: Punctuation appears as words
**Symptom:** "Hello comma world" → "Hello comma world" (no transformation)

**Solution:** Verify daemon is running with updated binary:
```bash
systemctl --user status swictation-daemon
ls -lh /usr/local/lib/node_modules/swictation/lib/native/swictation-daemon.bin
```

Restart daemon if needed:
```bash
systemctl --user restart swictation-daemon
```

### Issue: Numbers not converting (v2)
**Symptom:** "five" → "5" (should pass through in v2)

**Solution:** This is expected behavior in v2. Use "number five" for digit conversion.

**Symptom:** "number forty two" → "number forty two" (no conversion)

**Solution:** Check logs for transformation errors:
```bash
journalctl --user -u swictation-daemon | grep -i "transform\|error" | tail -20
```

### Issue: Hash/Plus/Equals not converting (v2)
**Symptom:** "hash tag" → "# tag" (should be "hash tag" in v2)

**Solution:** This is expected v2 behavior. Use explicit phrases:
- "hash sign tag" → "# tag"
- "plus sign x" → "+ x"
- "equals sign value" → "= value"

### Issue: Spacing problems between chunks
**Symptom:** "hello world.testing" (no space after period)

**Solution:** Fixed in v1.1.0. Ensure you're running latest daemon binary.

---

## 📚 Related Documentation

- **[Text Transform v2 PRD](./specs/text-transform-v2-prd.md)** - Complete v2 specification
- **[Parakeet-TDT Patterns](./parakeet-tdt-patterns.md)** - How the STT model transcribes speech
- **[VAD Chunk Spacing Analysis](./vad-chunk-spacing-analysis.md)** - Technical analysis of spacing issues

---

## 🎉 Credits

Secretary Mode was designed based on classic stenography practices and real-world testing with NVIDIA Parakeet-TDT speech-to-text models.

**Contributors:**
- Voice testing and feedback from real users
- Inspired by Dragon NaturallySpeaking's dictation commands
- Built on MidStream temporal computing framework

**Version:** 2.0.0 (November 2025) - Three-Layer Architecture
