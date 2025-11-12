# Secretary Dictation Mode

**Natural language dictation inspired by 1950s secretary stenography**

Secretary Mode is the first transformation mode in Swictation, designed for natural prose dictation where you speak punctuation, formatting, and special commands exactly as you would dictate to a secretary.

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

## 📋 Complete Feature List

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
| dash / hyphen | - | "Well dash known" → "Well-known" |
| ellipsis / three dots | ... | "Wait ellipsis" → "Wait..." |

### B. Parentheses & Brackets

Opening brackets/parentheses preserve space before them. Closing ones attach to previous word:

| Voice Command | Output | Example |
|--------------|--------|---------|
| open paren / open parenthesis / open parentheses | ( | "Text open paren note close paren" → "Text (note)" |
| close paren / close parenthesis / close parentheses | ) | - |
| open bracket | [ | "Array open bracket close bracket" → "Array []" |
| close bracket | ] | - |
| open brace | { | "Object open brace close brace" → "Object {}" |
| close brace | } | - |

### C. Quotes (Stateful Toggle)

Quotes intelligently toggle between opening and closing. First word inside quotes is automatically capitalized:

| Voice Command | Output | Example |
|--------------|--------|---------|
| quote | " (toggles) | "She said quote hello quote" → "She said \"Hello\"" |
| open quote | " | - |
| close quote | " | - |
| single quote | ' (toggles) | "It quote s nice" → "It's nice" |
| apostrophe | ' (attach) | For contractions: "don apostrophe t" → "don't" |

### D. Special Symbols

| Voice Command | Output | Example |
|--------------|--------|---------|
| dollar sign | $ | "dollar sign fifty" → "$ Fifty" |
| percent sign / percent | % | "fifty percent" → "Fifty %" |
| at sign | @ | "Email at sign example" → "Email @ example" |
| ampersand | & | "You ampersand me" → "You & me" |
| asterisk | * | "Note asterisk" → "Note *" |
| hash / hashtag / pound | # | "hash trending" → "# Trending" |
| forward slash / slash | / | "And slash or" → "And / or" |
| backslash | \ | "Path backslash file" → "Path \ file" |

### E. Math Symbols

| Voice Command | Output | Example |
|--------------|--------|---------|
| plus | + | "x plus y" → "X + y" |
| equals / equal sign | = | "a equals b" → "A = b" |
| times / multiply | × | "two times three" → "Two × three" |

### F. Formatting Commands

| Voice Command | Output | Example |
|--------------|--------|---------|
| new line | \n | "Line one new line line two" → "Line one\nline two" |
| new paragraph | \n\n | "Para one new paragraph para two" → "Para one\n\npara two" |
| tab | \t | "Indent tab text" → "Indent\ttext" |

### G. Abbreviations

Automatically expands common titles and abbreviations with proper punctuation:

| Voice Command | Output | Example |
|--------------|--------|---------|
| mister / mr | Mr. | "mister smith" → "Mr. Smith" |
| missus / mrs | Mrs. | "missus jones" → "Mrs. Jones" |
| doctor / dr | Dr. | "doctor brown" → "Dr. Brown" |
| ms | Ms. | "ms davis" → "Ms. Davis" |
| et cetera | etc. | "and so on et cetera" → "And so on etc." |
| versus | vs. | "team a versus team b" → "Team A vs. team B" |
| post script | P.S. | "post script thanks" → "P.S. Thanks" |

**Note:** Proper nouns after titles are automatically capitalized (see Capitalization section below).

### H. Number Conversion

Convert spoken numbers to digits using the **"number [words]"** command:

| Voice Command | Output | Range |
|--------------|--------|-------|
| number zero | 0 | - |
| number forty two | 42 | 0-99 |
| number two hundred fifty six | 256 | 0-999 |
| number nineteen fifty | 1950 | Year patterns (teen+decade) |

**Year Pattern:** "number" + teen (10-19) + decade (10-90) automatically creates years:
- "number nineteen fifty" → "1950"
- "number twenty twenty four" → "2024"

### I. Capitalization Modes

#### Caps Mode (Toggle)
Turn on/off uppercase mode for multiple words:

```
"caps on hello world caps off" → "HELLO WORLD"
"normal caps on loud caps off quiet" → "Normal LOUD quiet"
```

#### All Caps (Single Word)
Capitalize a single word (for acronyms):

```
"the all caps fbi investigated" → "The FBI investigated"
"all caps nasa launched" → "NASA launched"
```

#### Capital Letter Command (Proper Nouns)
Explicitly capitalize specific words with **"capital [letter] [word]"**:

```
"capital r robert smith" → "Robert smith"
"my name is capital j jones" → "My name is Jones"
"capital n new capital y york" → "New York"
```

**Use case:** When STT doesn't recognize proper nouns, you can manually capitalize them.

### J. Automatic Capitalization

Secretary Mode automatically capitalizes in these contexts:

#### 1. First-Person Pronoun "I"
```
"i am here" → "I am here"
"yes i am" → "Yes I am"
"i'm happy" → "I'm happy"
"i'll go" → "I'll go"
```

#### 2. Sentence Starts
After period, exclamation point, or question mark:

```
"hello period world" → "Hello. World"
"stop exclamation point go" → "Stop! Go"
"why question mark because" → "Why? Because"
```

#### 3. After Opening Quotes
First word inside quotes is capitalized:

```
"she said quote hello world quote" → "She said \"Hello world\""
"quote attention quote she yelled" → "\"Attention\" she yelled"
```

#### 4. After Titles (Proper Nouns)
Words after Mr., Mrs., Dr., Ms. are capitalized (unless common words like "and", "or"):

```
"mr smith arrived" → "Mr. Smith arrived"
"dr jones and dr brown" → "Dr. Jones and Dr. Brown"
"mr and mrs wilson" → "Mr. and Mrs. Wilson" (doesn't capitalize "and")
```

---

## 🚀 Usage Examples

### Example 1: Formal Letter
**Voice:**
```
"Dear mr smith comma new paragraph I am writing to confirm our meeting
on number twenty first period Please let me know if this works period
new paragraph Sincerely comma new line John"
```

**Output:**
```
Dear Mr. Smith,

I am writing to confirm our meeting on 21. Please let me know if this works.

Sincerely,
John
```

### Example 2: Meeting Notes
**Voice:**
```
"Meeting notes colon new line Action items colon new line
dash Review Q number four results open paren due friday close paren new line
dash Contact dr johnson at sign example dot com new line
dash Budget colon dollar sign fifty thousand"
```

**Output:**
```
Meeting notes:
Action items:
- Review Q 4 results (due friday)
- Contact Dr. Johnson @ example.com
- Budget: $ Fifty thousand
```

### Example 3: Email with Quote
**Voice:**
```
"Hi comma new paragraph As you mentioned comma quote we need to prioritize
quality over speed quote period I completely agree exclamation point
new paragraph Best comma new line Sarah"
```

**Output:**
```
Hi,

As you mentioned, "We need to prioritize quality over speed". I completely agree!

Best,
Sarah
```

---

## 🎯 Best Practices

### 1. Speak Naturally
Don't rush. Speak commands clearly with natural pauses:
- ✅ "Hello comma world period"
- ❌ "Hellocommaworld.period" (too fast)

### 2. Use Consistent Commands
Stick to one variant of each command:
- Use "comma" consistently (not switching between "comma" and other variants)
- Use "period" or "full stop" consistently

### 3. Proper Noun Capitalization
For proper nouns, use one of these strategies:

**Strategy A: Title + Name** (automatic capitalization)
```
"mr smith" → "Mr. Smith" ✓
"dr jones" → "Dr. Jones" ✓
```

**Strategy B: Capital Letter Command** (explicit capitalization)
```
"capital r robert capital s smith" → "Robert Smith" ✓
"capital n new capital y york" → "New York" ✓
```

### 4. Year Numbers
Use the teen+decade pattern for years:
```
"number nineteen fifty" → "1950" ✓
"number twenty twenty four" → "2024" ✓
```

---

## 🔧 Technical Details

### Architecture

Secretary Mode is implemented in the **MidStream text-transform** crate:
- **Rules:** `/external/midstream/crates/text-transform/src/rules.rs` (60+ static mappings)
- **Transform:** `/external/midstream/crates/text-transform/src/lib.rs` (capitalization, state tracking, number conversion)
- **Pipeline:** `/rust-crates/swictation-daemon/src/pipeline.rs` (integrates transformation after STT)

### Performance
- **Latency:** <1µs per transformation (HashMap O(1) lookup)
- **Memory:** Static rules (zero allocation)
- **Target:** <5ms total transformation latency

### How It Works

```
[Microphone] → [VAD] → [STT] → [Transform] → [Text Injection]
                                      ↓
                           Secretary Mode Rules
```

1. **VAD** detects speech segments (0.8s silence threshold)
2. **STT** transcribes to lowercase text: "mr smith said comma hello"
3. **Transform** applies secretary mode rules:
   - "mr" → "Mr." (abbreviation)
   - "smith" → "Smith" (capitalize after title)
   - "comma" → "," (punctuation)
   - "hello" → "hello" (keep lowercase mid-sentence)
4. **Result:** "Mr. Smith said, hello"
5. **Text Injection** types it into active window

### Spacing Between VAD Chunks

Secretary Mode automatically adds trailing spaces between VAD chunks:
```
Chunk 1: "hello world."  [0.8s silence]
Chunk 2: "testing"       [0.8s silence]
Result:  "hello world. testing"  ✓ (space added)
```

---

## 🚧 Limitations & Future Work

### Current Limitations
- **No context awareness:** Can't distinguish "period" (punctuation) vs "period" (time interval)
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

### Issue: Proper nouns not capitalized
**Symptom:** "mr smith" → "Mr. smith" (lowercase "smith")

**Solution:** This was fixed in v1.1.0. Update to latest version:
```bash
npm install -g swictation@latest
systemctl --user restart swictation-daemon
```

### Issue: Numbers not converting
**Symptom:** "number forty two" → "number forty two" (no conversion)

**Solution:** Check logs for transformation errors:
```bash
journalctl --user -u swictation-daemon | grep -i "transform\|error" | tail -20
```

### Issue: Spacing problems between chunks
**Symptom:** "hello world.testing" (no space after period)

**Solution:** Fixed in v1.1.0. Ensure you're running latest daemon binary.

---

## 📚 Related Documentation

- **[Parakeet-TDT Patterns](./parakeet-tdt-patterns.md)** - How the STT model transcribes speech
- **[VAD Chunk Spacing Analysis](./vad-chunk-spacing-analysis.md)** - Technical analysis of spacing issues
- **[Text Transformation Architecture](./text-transformation-architecture.md)** - Implementation details (TODO)

---

## 🎉 Credits

Secretary Mode was designed based on classic stenography practices and real-world testing with NVIDIA Parakeet-TDT speech-to-text models.

**Contributors:**
- Voice testing and feedback from real users
- Inspired by Dragon NaturallySpeaking's dictation commands
- Built on MidStream temporal computing framework

**Version:** 1.1.0 (November 2025)
