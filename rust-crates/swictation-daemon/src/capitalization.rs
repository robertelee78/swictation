/// Secretary Mode Capitalization Rules
/// Per docs/secretary-mode.md Section J

/// Apply automatic capitalization rules to transformed text
pub fn apply_capitalization(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true; // Start with capital
    let mut in_quote = false;
    let mut prev_was_title = false;

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
            prev_was_title = false;
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
                let is_standalone = chars.peek().map_or(true, |&next| !next.is_alphabetic());

                // Look back to see if previous char was non-alphabetic
                let prev_char = result.chars().last();
                let prev_is_boundary = prev_char.map_or(true, |c| !c.is_alphabetic());

                if is_standalone && prev_is_boundary {
                    result.push('I');
                } else {
                    result.push(ch);
                }
            } else {
                // Check if we're starting a title (mr., mrs., dr., ms.)
                // Look for word boundary before this letter
                let prev_char = result.chars().last();
                let at_word_start = prev_char.map_or(true, |c| c.is_whitespace());

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
            prev_was_title = true;
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
        if i + 1 < words.len() && words[i] == "all" && words[i + 1] == "caps" {
            if i + 2 < words.len() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&words[i + 2].to_uppercase());
                i += 3; // Skip "all", "caps", and word
                continue;
            }
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
}
