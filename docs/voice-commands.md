# Voice Commands Reference

Guide to using voice dictation effectively for coding and general text input with Swictation.

> **Note:** This is a reference guide for natural speech-to-text. Text transformation features (e.g., "new line" → `\n`) are planned for future releases.

---

## General Usage

### Basic Dictation

Swictation transcribes your natural speech to text:

```
YOU SAY:           "Hello world, this is a test."
SWICTATION TYPES:  Hello world, this is a test.
```

**Tips:**
- Speak clearly at normal pace
- Pause briefly at sentence boundaries
- Use natural intonation

### Punctuation

**Currently:** Speak punctuation explicitly:

```
YOU SAY:           "Hello comma world exclamation mark"
SWICTATION TYPES:  Hello, world!
```

**Supported punctuation:**
- Period (`.`) - "period" or "dot"
- Comma (`,`) - "comma"
- Question mark (`?`) - "question mark"
- Exclamation mark (`!`) - "exclamation mark" or "exclamation point"
- Colon (`:`) - "colon"
- Semicolon (`;`) - "semicolon"
- Dash (`-`) - "dash" or "hyphen"
- Underscore (`_`) - "underscore"

---

## Coding with Voice

### Python Examples

**Variable assignment:**
```
YOU SAY:           "x equals ten"
SWICTATION TYPES:  x equals 10
```

**Function definition:**
```
YOU SAY:           "def hello underscore world open parenthesis close parenthesis colon"
SWICTATION TYPES:  def hello_world():
```

**For loop:**
```
YOU SAY:           "for i in range open parenthesis ten close parenthesis colon"
SWICTATION TYPES:  for i in range(10):
```

### JavaScript Examples

**Const declaration:**
```
YOU SAY:           "const data equals open bracket close bracket semicolon"
SWICTATION TYPES:  const data = [];
```

**Arrow function:**
```
YOU SAY:           "const add equals open parenthesis a comma b close parenthesis arrow open brace"
SWICTATION TYPES:  const add = (a, b) => {
```

**Console log:**
```
YOU SAY:           "console dot log open parenthesis quote hello quote close parenthesis semicolon"
SWICTATION TYPES:  console.log("hello");
```

---

## Special Characters

### Common Symbols

| Symbol | Voice Command | Example Usage |
|--------|---------------|---------------|
| `{` | "open brace" | Function body |
| `}` | "close brace" | End function |
| `[` | "open bracket" | Array literal |
| `]` | "close bracket" | End array |
| `(` | "open parenthesis" | Function call |
| `)` | "close parenthesis" | End call |
| `<` | "less than" | Comparison |
| `>` | "greater than" | Comparison |
| `=` | "equals" | Assignment |
| `==` | "double equals" | Equality |
| `===` | "triple equals" | Strict equality |
| `=>` | "arrow" | Arrow function |
| `+` | "plus" | Addition |
| `-` | "minus" | Subtraction |
| `*` | "asterisk" or "star" | Multiplication |
| `/` | "slash" | Division |
| `%` | "percent" | Modulo |
| `&` | "ampersand" or "and" | Logical AND |
| `\|` | "pipe" or "bar" | Logical OR |
| `!` | "exclamation" or "bang" | Not |
| `@` | "at sign" | Decorator |
| `#` | "hash" or "pound" | Comment |
| `$` | "dollar sign" | Variable |
| `^` | "caret" | XOR |
| `~` | "tilde" | Home directory |
| `` ` `` | "backtick" | Template literal |
| `'` | "single quote" | String |
| `"` | "double quote" or "quote" | String |

---

## Workflow Tips

### Starting and Stopping (VAD Streaming)

**Press `$mod+Shift+d` once:**
- Starts continuous recording
- Speak naturally with pauses

**While recording:**
- Text appears automatically after 2-second pauses
- No need to stop between sentences
- Just keep speaking and pausing naturally

**Press `$mod+Shift+d` again:**
- Stops recording
- Final segment (if any) is transcribed

### Best Practices

**DO:**
- ✅ Speak in short phrases (5-10 words)
- ✅ Pause briefly between commands
- ✅ Use consistent naming conventions
- ✅ Speak punctuation explicitly
- ✅ Test in simple editor first (kate, gedit)

**DON'T:**
- ❌ Speak for >30 seconds continuously
- ❌ Rush through code dictation
- ❌ Forget to say punctuation
- ❌ Assume text transformations work (not implemented yet)

---

## Language-Specific Patterns

### Python

**Class definition:**
```
YOU SAY:
"class user colon"
"    def underscore underscore init underscore underscore open parenthesis self comma name close parenthesis colon"
"        self dot name equals name"

SWICTATION TYPES:
class User:
    def __init__(self, name):
        self.name = name
```

**List comprehension:**
```
YOU SAY:
"squares equals open bracket x star star two for x in range open parenthesis ten close parenthesis close bracket"

SWICTATION TYPES:
squares = [x**2 for x in range(10)]
```

### Bash

**Shebang:**
```
YOU SAY:           "hash bang slash bin slash bash"
SWICTATION TYPES:  #!/bin/bash
```

**Pipe command:**
```
YOU SAY:           "cat file dot txt pipe grep quote error quote"
SWICTATION TYPES:  cat file.txt | grep "error"
```

### Git Commands

```
YOU SAY:           "git add period"
SWICTATION TYPES:  git add .

YOU SAY:           "git commit hyphen m quote initial commit quote"
SWICTATION TYPES:  git commit -m "initial commit"
```

---

## Editing Strategies

### Dictate Skeleton, Fill Details

**Strategy:** Dictate structure first, then manually edit details

```
DICTATE:           "def process underscore data open parenthesis data close parenthesis colon"
                   "    pass"

MANUALLY EDIT:     Add implementation details with keyboard
```

### Use Templates

**Strategy:** Create code templates, dictate minimal text

```
# Template in .vimrc or .emacs
iabbrev pyfor for i in range():<CR>    pass

DICTATE:           "pyfor"  (expands to template)
MANUALLY EDIT:     Fill in range and body
```

---

## Future Features (Planned)

### Text Transformations

**Coming soon:** Intelligent code-specific transforms

```
YOU SAY:           "new line"
SWICTATION TYPES:  \n

YOU SAY:           "indent"
SWICTATION TYPES:  [inserts 4 spaces]

YOU SAY:           "camel case user name"
SWICTATION TYPES:  userName

YOU SAY:           "snake case user name"
SWICTATION TYPES:  user_name
```

### Code Macros

**Coming soon:** High-level coding commands

```
YOU SAY:           "python function hello world"
SWICTATION TYPES:  def hello_world():
                       pass

YOU SAY:           "javascript arrow function add two numbers"
SWICTATION TYPES:  const addTwoNumbers = (a, b) => a + b;
```

### Language Detection

**Coming soon:** Auto-detect programming language context

```
[In Python file]
YOU SAY:           "for loop ten times"
SWICTATION TYPES:  for i in range(10):
                       pass

[In JavaScript file]
YOU SAY:           "for loop ten times"
SWICTATION TYPES:  for (let i = 0; i < 10; i++) {

                   }
```

---

## Performance Tips

### Optimize Latency

**Short utterances (<5s):** ~382ms latency
```
✅ GOOD: "const x equals ten semicolon"
```

**Long utterances (>20s):** ~420ms+ latency
```
⚠️  SLOWER: [30 seconds of continuous speech]
```

**Recommendation:** Speak in 5-10 second chunks

### Battery Optimization

**Voice Activity Detection (VAD)** skips silence:
- Automatic pause detection
- GPU only processes speech
- Battery savings on laptops

**Tip:** Natural pauses are good!

---

## Accessibility Features

### RSI/Carpal Tunnel

Swictation reduces typing strain:

**Before:**
- Type 100+ words/minute
- Repetitive wrist motion
- Pain after 2-3 hours

**After with Swictation:**
- Dictate at natural speech pace
- No wrist motion
- Work full day comfortably

### Hands-Free Coding

**Use cases:**
- Brainstorming algorithms (speak pseudocode)
- Code reviews (dictate comments)
- Documentation (dictate explanations)
- Learning (dictate notes while coding)

---

## Examples from Real Usage

### Documenting a Function

```
YOU SAY:
"triple quote"
"Calculate the factorial of a number period"
"Args colon"
"    n colon int hyphen the number to calculate factorial for"
"Returns colon"
"    int hyphen the factorial result"
"triple quote"

SWICTATION TYPES:
"""
Calculate the factorial of a number.
Args:
    n: int - the number to calculate factorial for
Returns:
    int - the factorial result
"""
```

### Writing Tests

```
YOU SAY:
"def test underscore add underscore numbers open parenthesis close parenthesis colon"
"    assert add open parenthesis two comma three close parenthesis equals equals five"

SWICTATION TYPES:
def test_add_numbers():
    assert add(2, 3) == 5
```

---

## Known Limitations

**Current version limitations:**
- ❌ No automatic punctuation restoration
- ❌ No automatic capitalization
- ❌ No code-specific transformations
- ❌ No macro expansion
- ❌ Limited context awareness

**Workarounds:**
- Speak punctuation explicitly
- Use editor's auto-capitalization
- Create editor macros/snippets
- Manual editing after dictation

---

## FAQ

### Q: Can I dictate emojis?
**A:** Yes! Say "emoji" + description:
```
YOU SAY:           "emoji rocket"
SWICTATION TYPES:  🚀
```
(Depends on STT model's training)

### Q: How do I dictate numbers?
**A:** Speak naturally:
```
YOU SAY:           "x equals forty-two"
SWICTATION TYPES:  x = 42
```

### Q: Can I dictate in other languages?
**A:** Yes! Canary-1B-Flash supports multilingual:
- English, Spanish, German, French
- Mandarin, Hindi, Japanese, Korean
- And more!

### Q: How accurate is it?
**A:** 5.77% WER (Word Error Rate) = 94.23% accuracy
- Excellent for 1B parameter model
- Better with clear speech
- May struggle with technical jargon

---

## Community Tips

Share your voice coding patterns on GitHub!

**Example patterns:**
- "new function" → full function template
- "comment section" → documentation block
- "error handling" → try/catch block

---

**Last Updated:** 2025-10-30

**Feedback:** Help improve this guide! Submit voice command patterns that work well for you.
