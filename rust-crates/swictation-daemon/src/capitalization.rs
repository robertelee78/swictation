/// Secretary Mode Capitalization Rules
/// Per docs/secretary-mode.md Section J
/// Apply automatic capitalization rules to transformed text
pub fn apply_capitalization(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true; // Start with capital
    let mut in_quote = false;

    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        // Check if we're entering or leaving a quote
        if ch == '"' {
            in_quote = !in_quote;
            result.push(ch);
            if in_quote {
                capitalize_next = true; // Capitalize first word in quote
            }
            continue;
        }

        // Handle whitespace
        if ch.is_whitespace() {
            result.push(ch);
            continue;
        }

        // Check for sentence-ending punctuation
        if ch == '.' || ch == '!' || ch == '?' {
            result.push(ch);
            capitalize_next = true;
            continue;
        }

        // Handle other punctuation
        if !ch.is_alphabetic() {
            result.push(ch);
            continue;
        }

        // Now we have a letter
        if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            capitalize_next = false;
        } else {
            // Check if this is "i" standalone (first person pronoun)
            if ch == 'i' {
                // Look ahead to see if next char is non-alphabetic (word boundary)
                let is_standalone = chars.peek().is_none_or(|&next| !next.is_alphabetic());

                // Look back to see if previous char was non-alphabetic
                let prev_char = result.chars().last();
                let prev_is_boundary = prev_char.is_none_or(|c| !c.is_alphabetic());

                if is_standalone && prev_is_boundary {
                    result.push('I');
                } else {
                    result.push(ch);
                }
            } else {
                // Check if we're starting a title (mr., mrs., dr., ms.)
                // Look for word boundary before this letter
                let prev_char = result.chars().last();
                let at_word_start = prev_char.is_none_or(|c| c.is_whitespace());

                if at_word_start && (ch == 'm' || ch == 'd') {
                    // Peek ahead to see if this is a title
                    let remaining: String = chars.clone().collect();
                    let next_word = format!(
                        "{}{}",
                        ch,
                        remaining.split_whitespace().next().unwrap_or("")
                    );

                    if next_word == "mr."
                        || next_word == "mrs."
                        || next_word == "ms."
                        || next_word == "dr."
                    {
                        result.push(ch.to_uppercase().next().unwrap_or(ch));
                    } else {
                        result.push(ch);
                    }
                } else {
                    result.push(ch);
                }
            }
        }

        // Check if we just wrote a title (Mr., Mrs., Dr., Ms.)
        if result.ends_with("Mr.")
            || result.ends_with("Mrs.")
            || result.ends_with("Dr.")
            || result.ends_with("Ms.")
        {
            capitalize_next = true; // Capitalize next word after title
        }
    }

    result
}

/// Process explicit capital commands like "capital r robert"
pub fn process_capital_commands(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;

    while i < words.len() {
        // Check for "capital [letter] [word]" pattern
        if i + 2 < words.len() && words[i] == "capital" {
            let letter = words[i + 1];
            let word = words[i + 2];

            // Check if it's a single letter command
            if letter.len() == 1 && letter.chars().next().unwrap().is_alphabetic() {
                // Capitalize the word
                if !result.is_empty() {
                    result.push(' ');
                }
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_uppercase().next().unwrap_or(first));
                    result.push_str(&chars.collect::<String>());
                }
                i += 3; // Skip "capital", letter, and word
                continue;
            }
        }

        // Check for "all caps [word]" pattern
        if i + 1 < words.len() && words[i] == "all" && words[i + 1] == "caps" && i + 2 < words.len()
        {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&words[i + 2].to_uppercase());
            i += 3; // Skip "all", "caps", and word
            continue;
        }

        // Regular word
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(words[i]);
        i += 1;
    }

    result
}

/// Strip 0.6B model's built-in ITN (Inverse Text Normalization) completely.
///
/// The 0.6B model has built-in ITN that CANNOT be disabled at inference time.
/// This converts punctuation words to symbols inconsistently:
/// - "comma" → "," (word replaced with symbol)
/// - "period" → "period." (word kept + symbol added)
/// - "semicolon" → ",;" (spurious comma added + semicolon)
///
/// **Solution**: Strip ALL ITN effects to produce raw text like 1.1B model outputs.
/// Then let Secretary Mode handle the word→symbol conversion consistently.
///
/// # Processing Steps
/// 1. Lowercase everything
/// 2. Convert ALL punctuation symbols to word equivalents
/// 3. Remove spurious "comma" before other punctuation (common 0.6B artifact)
/// 4. Remove consecutive duplicate punctuation words
/// 5. Clean up whitespace
///
/// # Examples
/// ```
/// use swictation_daemon::capitalization::normalize_0_6b_punctuation;
///
/// // Symbol → word conversion
/// assert_eq!(normalize_0_6b_punctuation("Hello, world"), "hello comma world");
///
/// // Duplicate removal (word + symbol → single word)
/// assert_eq!(normalize_0_6b_punctuation("hello period."), "hello period");
///
/// // Spurious comma removal
/// assert_eq!(normalize_0_6b_punctuation("First,; second."), "first semicolon second period");
///
/// // Full normalization
/// assert_eq!(normalize_0_6b_punctuation("Hello, world period."), "hello comma world period");
/// ```
pub fn normalize_0_6b_punctuation(text: &str) -> String {
    // Step 1: Lowercase everything (model adds capitalization we'll reapply later)
    let text = text.to_lowercase();

    // Use numeric markers to avoid substring collision (e.g., "period" inside "⟪period⟫")
    // Markers: ⟪1⟫=comma, ⟪2⟫=period, ⟪3⟫=question, ⟪4⟫=exclamation, ⟪5⟫=semicolon, ⟪6⟫=colon, ⟪7⟫=dash, ⟪8⟫=ellipsis

    // Step 2: Convert punctuation WORDS to unique markers (before symbol conversion)
    // Multi-word punctuation must come first (before single-word "mark" matches "exclamation mark")
    let text = text
        // Multi-word punctuation
        .replace("exclamation point", "⟪4⟫")
        .replace("exclamation mark", "⟪4⟫")
        .replace("question mark", "⟪3⟫")
        .replace("full stop", "⟪2⟫")
        .replace("semi colon", "⟪5⟫")
        .replace("three dots", "⟪8⟫")
        // Single-word punctuation
        .replace("ellipsis", "⟪8⟫")
        .replace("semicolon", "⟪5⟫")
        .replace("period", "⟪2⟫")
        .replace("comma", "⟪1⟫")
        .replace("colon", "⟪6⟫")
        .replace("dash", "⟪7⟫");

    // Step 3: Convert ALL punctuation SYMBOLS to markers
    // Order matters: longer sequences first
    let text = text
        .replace("...", " ⟪8⟫ ")
        .replace("--", " ⟪7⟫ ") // Em-dash alternative
        .replace(',', " ⟪1⟫ ")
        .replace('.', " ⟪2⟫ ")
        .replace('?', " ⟪3⟫ ")
        .replace('!', " ⟪4⟫ ")
        .replace(';', " ⟪5⟫ ")
        .replace(':', " ⟪6⟫ ")
        .replace('-', " ⟪7⟫ ");

    // Step 4: Split into tokens and clean up
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut result: Vec<&str> = Vec::with_capacity(tokens.len());

    // Punctuation markers we recognize
    let punct_markers = ["⟪1⟫", "⟪2⟫", "⟪3⟫", "⟪4⟫", "⟪5⟫", "⟪6⟫", "⟪7⟫", "⟪8⟫"];

    for (i, token) in tokens.iter().enumerate() {
        // Skip consecutive duplicate punctuation markers
        if i > 0 && *token == tokens[i - 1] && punct_markers.contains(token) {
            continue;
        }

        // Skip spurious comma (⟪1⟫) if followed by another punctuation marker
        // (Common 0.6B artifact: ",;" → "⟪1⟫ ⟪5⟫" → "⟪5⟫")
        if *token == "⟪1⟫" {
            if let Some(&next) = tokens.get(i + 1) {
                if punct_markers.contains(&next) && next != "⟪1⟫" {
                    continue; // Skip this spurious comma
                }
            }
        }

        // Skip spurious period (⟪2⟫) if PRECEDED by exclamation (⟪4⟫) or question (⟪3⟫)
        // (Common 0.6B artifact: "exclamation point." → "⟪4⟫ ⟪2⟫" → "⟪4⟫")
        if *token == "⟪2⟫" {
            if let Some(&prev) = result.last() {
                if prev == "⟪4⟫" || prev == "⟪3⟫" {
                    continue; // Skip this spurious period
                }
            }
        }

        result.push(token);
    }

    // Step 5: Convert markers back to canonical words
    result
        .join(" ")
        .replace("⟪1⟫", "comma")
        .replace("⟪2⟫", "period")
        .replace("⟪3⟫", "question mark")
        .replace("⟪4⟫", "exclamation point")
        .replace("⟪5⟫", "semicolon")
        .replace("⟪6⟫", "colon")
        .replace("⟪7⟫", "dash")
        .replace("⟪8⟫", "ellipsis")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_capitalization() {
        assert_eq!(apply_capitalization("hello, world."), "Hello, world.");
        assert_eq!(apply_capitalization("hello. world"), "Hello. World");
        assert_eq!(apply_capitalization("why? because!"), "Why? Because!");
    }

    #[test]
    fn test_i_pronoun() {
        assert_eq!(apply_capitalization("i am here"), "I am here");
        assert_eq!(apply_capitalization("yes i am"), "Yes I am");
        assert_eq!(apply_capitalization("i'm happy"), "I'm happy");
    }

    #[test]
    fn test_quotes() {
        assert_eq!(
            apply_capitalization("she said \"hello world\""),
            "She said \"Hello world\""
        );
        assert_eq!(
            apply_capitalization("\"attention\" she yelled"),
            "\"Attention\" she yelled"
        );
    }

    #[test]
    fn test_titles() {
        assert_eq!(apply_capitalization("mr. smith"), "Mr. Smith");
        assert_eq!(
            apply_capitalization("dr. jones and dr. brown"),
            "Dr. Jones and Dr. Brown"
        );
    }

    #[test]
    fn test_capital_commands() {
        assert_eq!(process_capital_commands("capital r robert"), "Robert");
        assert_eq!(
            process_capital_commands("my name is capital j jones"),
            "my name is Jones"
        );
        assert_eq!(process_capital_commands("all caps fbi"), "FBI");
    }

    // ========================================
    // Tests for 0.6B model punctuation normalization
    // ========================================

    #[test]
    fn test_normalize_0_6b_word_exists_remove_symbol() {
        // When the word form exists, the symbol is redundant and should be removed
        // This handles: "period" → "period." where model adds trailing period
        assert_eq!(normalize_0_6b_punctuation("hello period."), "hello period");
        assert_eq!(
            normalize_0_6b_punctuation("what question mark?"),
            "what question mark"
        );
        assert_eq!(
            normalize_0_6b_punctuation("stop exclamation point!"),
            "stop exclamation point"
        );
    }

    #[test]
    fn test_normalize_0_6b_word_missing_add_word() {
        // When the word form doesn't exist, convert symbol to word
        // This handles: "comma" → "," where model replaced word with symbol
        assert_eq!(normalize_0_6b_punctuation("hello, world"), "hello comma world");
        assert_eq!(normalize_0_6b_punctuation("what?"), "what question mark");
        assert_eq!(normalize_0_6b_punctuation("stop!"), "stop exclamation point");
        assert_eq!(normalize_0_6b_punctuation("note: important"), "note colon important");
        assert_eq!(
            normalize_0_6b_punctuation("first; second"),
            "first semicolon second"
        );
    }

    #[test]
    fn test_normalize_0_6b_mixed_behavior() {
        // The actual 0.6B model behavior: inconsistent handling
        // "hello comma world period" → "Hello, world period."
        // - "comma" was converted to "," (need to add word back)
        // - "period" stayed as word but "." was added (need to remove symbol)
        assert_eq!(
            normalize_0_6b_punctuation("Hello, world period."),
            "hello comma world period"
        );
    }

    #[test]
    fn test_normalize_0_6b_lowercase() {
        // Should lowercase everything (model adds capitalization we reapply later)
        assert_eq!(normalize_0_6b_punctuation("Hello World"), "hello world");
        assert_eq!(normalize_0_6b_punctuation("HELLO"), "hello");
    }

    #[test]
    fn test_normalize_0_6b_whitespace_cleanup() {
        // Should normalize whitespace
        assert_eq!(
            normalize_0_6b_punctuation("hello   world"),
            "hello world"
        );
        assert_eq!(
            normalize_0_6b_punctuation("  hello  world  "),
            "hello world"
        );
    }

    #[test]
    fn test_normalize_0_6b_no_punctuation() {
        // No punctuation should pass through unchanged (except lowercase)
        assert_eq!(normalize_0_6b_punctuation("hello world"), "hello world");
        assert_eq!(normalize_0_6b_punctuation("Hello World"), "hello world");
    }

    #[test]
    fn test_normalize_0_6b_full_stop_variant() {
        // "full stop" is normalized to canonical "period"
        // word + symbol redundancy: "full stop." → "period" (both merged)
        assert_eq!(
            normalize_0_6b_punctuation("hello full stop."),
            "hello period"
        );
    }

    #[test]
    fn test_normalize_0_6b_exclamation_mark_variant() {
        // "exclamation mark" is normalized to canonical "exclamation point"
        // Also removes spurious trailing symbol
        assert_eq!(
            normalize_0_6b_punctuation("wow exclamation mark!"),
            "wow exclamation point"
        );
    }

    #[test]
    fn test_normalize_0_6b_complex_sentence() {
        // Test a more complex sentence with multiple punctuation
        // Input: User says "Hello comma how are you question mark I am fine period"
        // Model outputs: "Hello, how are you question mark? I am fine period."
        // (comma converted to symbol, question mark word + symbol, period word + symbol)
        assert_eq!(
            normalize_0_6b_punctuation("Hello, how are you question mark? I am fine period."),
            "hello comma how are you question mark i am fine period"
        );
    }

    #[test]
    fn test_normalize_0_6b_secretary_mode_examples() {
        // Test examples from docs/secretary-mode.md Section A

        // "Hello comma world" - model might output "Hello, world"
        assert_eq!(
            normalize_0_6b_punctuation("Hello, world"),
            "hello comma world"
        );

        // "End period" - model might output "End." or "End period."
        assert_eq!(normalize_0_6b_punctuation("End."), "end period");
        assert_eq!(normalize_0_6b_punctuation("End period."), "end period");

        // "Why question mark" - model might output "Why?" or "Why question mark?"
        assert_eq!(normalize_0_6b_punctuation("Why?"), "why question mark");
        assert_eq!(
            normalize_0_6b_punctuation("Why question mark?"),
            "why question mark"
        );

        // "Stop exclamation point" - model might output "Stop!" or "Stop exclamation point!"
        // CRITICAL: Model may add trailing PERIOD instead of "!" - must strip it
        assert_eq!(normalize_0_6b_punctuation("Stop!"), "stop exclamation point");
        assert_eq!(
            normalize_0_6b_punctuation("Stop exclamation point!"),
            "stop exclamation point"
        );
        // BUG FIX: Model outputs "Stop exclamation point." with trailing period artifact
        assert_eq!(
            normalize_0_6b_punctuation("Stop exclamation point."),
            "stop exclamation point"
        );

        // "Note colon" - model might output "Note:" or "Note colon:"
        assert_eq!(normalize_0_6b_punctuation("Note:"), "note colon");
        assert_eq!(normalize_0_6b_punctuation("Note colon:"), "note colon");

        // "First semicolon second" - model might output "First; second" or "First semicolon; second"
        assert_eq!(
            normalize_0_6b_punctuation("First; second"),
            "first semicolon second"
        );
        assert_eq!(
            normalize_0_6b_punctuation("First semicolon; second"),
            "first semicolon second"
        );

        // CRITICAL BUG FIX: Spurious comma before semicolon
        // The original bug: user says "first semicolon second", model outputs "First,; second."
        // This must become "first semicolon second" (comma removed, period preserved if spoken)
        assert_eq!(
            normalize_0_6b_punctuation("First,; second"),
            "first semicolon second"
        );
        assert_eq!(
            normalize_0_6b_punctuation("First,; second."),
            "first semicolon second period"
        );
    }
}
