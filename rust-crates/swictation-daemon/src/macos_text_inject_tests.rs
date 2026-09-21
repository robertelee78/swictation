use super::*;

#[test]
fn posting_permission_recovery_only_dispatches_new_text() {
    let mut posted = Vec::new();
    let denied = post_text_batches(
        "discarded",
        || false,
        |chunk| {
            posted.push(String::from_utf16(chunk).unwrap());
            Ok(())
        },
    );
    assert!(denied
        .unwrap_err()
        .to_string()
        .contains("event-posting permission"));
    assert!(posted.is_empty());

    post_text_batches(
        "fresh",
        || true,
        |chunk| {
            posted.push(String::from_utf16(chunk).unwrap());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(posted, ["fresh"]);
}

#[test]
fn posting_permission_loss_stops_before_the_next_batch() {
    let text = "x".repeat(BATCH_UTF16_LIMIT + 5);
    let mut checks = 0;
    let mut posted = Vec::new();
    let result = post_text_batches(
        &text,
        || {
            checks += 1;
            checks == 1
        },
        |chunk| {
            posted.extend_from_slice(chunk);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert_eq!(checks, 2);
    assert_eq!(posted, vec![u16::from(b'x'); BATCH_UTF16_LIMIT]);
}

#[test]
fn posting_error_is_returned_without_dispatching_later_batches() {
    let text = "x".repeat(BATCH_UTF16_LIMIT + 5);
    let mut attempts = 0;
    let result = post_text_batches(
        &text,
        || true,
        |_| {
            attempts += 1;
            anyhow::bail!("event creation failed")
        },
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("event creation failed"));
    assert_eq!(attempts, 1);
}

#[test]
fn test_permission_check() {
    // This test just verifies the function can be called
    // Actual permission state depends on system configuration
    let has_permission = MacOSTextInjector::check_accessibility_permissions();
    println!("Accessibility permission (API): {}", has_permission);
}

#[test]
fn test_permission_validation() {
    // Test that the validation function works
    // This actually validates permission via CGEventTap
    let validated = MacOSTextInjector::validate_accessibility_permission();
    let api_says = MacOSTextInjector::check_accessibility_permissions();
    println!(
        "Accessibility permission - API: {}, Validated: {}",
        api_says, validated
    );

    // If API says yes but validation says no, we have stale permissions
    if api_says && !validated {
        println!("⚠️  STALE PERMISSION DETECTED: API reports granted but validation failed");
        println!("    This means the binary has changed and needs re-authorization");
    }
}

#[test]
fn test_injector_creation() {
    // Only test creation if permissions are granted
    match MacOSTextInjector::new() {
        Ok(_injector) => {
            println!("✅ Text injector created successfully");
        }
        Err(e) => {
            println!(
                "⚠️  Text injector creation failed (expected if no permissions): {}",
                e
            );
        }
    }
}

/// Regression: what is typed is exactly what was passed in.
///
/// This injector used to parse `<KEY:Cmd+C>` out of its input and post the
/// real key events, so dictating that phrase pressed Cmd+C in the focused
/// window. Every code unit must now survive to the keyboard as a character.
#[test]
fn test_typed_payload_is_the_input_verbatim() {
    for text in [
        "Copy this <KEY:Cmd+C>",
        "<KEY:Cmd+Shift+V>",
        "unterminated <KEY:Cmd+C",
        "plain text",
        // Longer than one CGEvent payload, so it spans several chunks
        "a marker <KEY:Cmd+A> buried in a sentence long enough to be split",
        // Non-BMP characters exercise the surrogate-pair boundary
        "emoji 🎤 and <KEY:Cmd+V> 👍🏽 mixed",
    ] {
        let typed = String::from_utf16(&utf16_chunks(text).concat())
            .expect("chunks must recombine into valid UTF-16");
        assert_eq!(typed, text, "injected payload must equal the input");
    }
}

#[test]
fn test_chunks_respect_cgevent_limits() {
    let text = "🎤".repeat(40) + &"x".repeat(100);
    let chunks = utf16_chunks(&text);

    for (i, chunk) in chunks.iter().enumerate() {
        assert!(
            chunk.len() <= BATCH_UTF16_LIMIT,
            "chunk {i} carries {} code units, over Apple's limit",
            chunk.len()
        );
        let is_last = i + 1 == chunks.len();
        if !is_last {
            assert!(
                !is_high_surrogate(*chunk.last().unwrap()),
                "chunk {i} ends on a high surrogate, splitting a character"
            );
        }
    }
}

#[test]
fn test_key_markers_are_typed_not_pressed() {
    let injector = match MacOSTextInjector::new() {
        Ok(inj) => inj,
        Err(_) => {
            println!("⚠️  Skipping test (no permissions)");
            return;
        }
    };

    for text in [
        "Copy this <KEY:Cmd+C>",
        "<KEY:Cmd+Shift+V>",
        // Previously aborted injection with "Malformed KEY marker"
        "unterminated <KEY:Cmd+C",
    ] {
        assert!(
            injector.inject_text(text).is_ok(),
            "literal injection should succeed for {text:?}"
        );
    }
}
