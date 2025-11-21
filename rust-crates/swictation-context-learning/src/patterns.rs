//! Context pattern extraction from segment history

use crate::Segment;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A context pattern discovered from segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPattern {
    pub pattern_type: PatternType,
    pub description: String,
    pub confidence: f64,
    pub support: usize, // Number of occurrences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    CoOccurrence {
        word_a: String,
        word_b: String,
        distance: usize, // Words apart
    },
    TemporalWindow {
        keywords: Vec<String>,
        time_window_seconds: i64,
    },
    TransformationSignal {
        low_transformations: bool,
        context_type: String,
    },
}

impl ContextPattern {
    pub fn to_pattern_string(&self) -> String {
        match &self.pattern_type {
            PatternType::CoOccurrence {
                word_a,
                word_b,
                distance,
            } => format!("co_occur:{}+{}@{}", word_a, word_b, distance),

            PatternType::TemporalWindow {
                keywords,
                time_window_seconds,
            } => format!("temporal:{}@{}s", keywords.join("+"), time_window_seconds),

            PatternType::TransformationSignal {
                low_transformations,
                context_type,
            } => format!(
                "transform:{}:{}",
                if *low_transformations { "low" } else { "high" },
                context_type
            ),
        }
    }
}

/// Extract context patterns from segments
pub fn extract_patterns(segments: &[Segment], context_window: usize) -> Result<Vec<ContextPattern>> {
    let mut patterns = Vec::new();

    // 1. Co-occurrence patterns
    patterns.extend(extract_cooccurrence_patterns(segments)?);

    // 2. Temporal window patterns
    patterns.extend(extract_temporal_patterns(segments, context_window)?);

    // 3. Transformation signal patterns
    patterns.extend(extract_transformation_patterns(segments)?);

    Ok(patterns)
}

/// Extract word co-occurrence patterns
fn extract_cooccurrence_patterns(segments: &[Segment]) -> Result<Vec<ContextPattern>> {
    let mut cooccur_map: HashMap<(String, String, usize), usize> = HashMap::new();

    for segment in segments {
        let words: Vec<String> = segment
            .text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        // Look for pairs within 5 words of each other
        for i in 0..words.len() {
            for j in (i + 1)..words.len().min(i + 6) {
                let word_a = words[i].clone();
                let word_b = words[j].clone();
                let distance = j - i;

                if word_a == word_b {
                    continue; // Skip self-pairs
                }

                let key = if word_a < word_b {
                    (word_a, word_b, distance)
                } else {
                    (word_b, word_a, distance)
                };

                *cooccur_map.entry(key).or_insert(0) += 1;
            }
        }
    }

    // Convert to patterns (filter low-frequency pairs)
    let min_support = 3;
    let patterns: Vec<ContextPattern> = cooccur_map
        .into_iter()
        .filter(|(_, count)| *count >= min_support)
        .map(|((word_a, word_b, distance), support)| {
            ContextPattern {
                pattern_type: PatternType::CoOccurrence {
                    word_a: word_a.clone(),
                    word_b: word_b.clone(),
                    distance,
                },
                description: format!("{} appears with {} ({} words apart)", word_a, word_b, distance),
                confidence: 0.8,
                support,
            }
        })
        .collect();

    Ok(patterns)
}

/// Extract temporal window patterns
fn extract_temporal_patterns(
    segments: &[Segment],
    window_size: usize,
) -> Result<Vec<ContextPattern>> {
    let mut patterns = Vec::new();

    for window in segments.windows(window_size) {
        // Collect all words in this window
        let mut word_freq: HashMap<String, usize> = HashMap::new();

        for segment in window {
            for word in segment.text.split_whitespace() {
                let word_lower = word.to_lowercase();
                *word_freq.entry(word_lower).or_insert(0) += 1;
            }
        }

        // Find frequently co-occurring words in this window
        let frequent_words: Vec<String> = word_freq
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .map(|(word, _)| word)
            .collect();

        if frequent_words.len() >= 2 {
            let time_window = if !window.is_empty() {
                (window.last().unwrap().timestamp - window.first().unwrap().timestamp)
                    .num_seconds()
            } else {
                0
            };

            patterns.push(ContextPattern {
                pattern_type: PatternType::TemporalWindow {
                    keywords: frequent_words.clone(),
                    time_window_seconds: time_window,
                },
                description: format!(
                    "Words {} appear together in {}s window",
                    frequent_words.join(", "),
                    time_window
                ),
                confidence: 0.75,
                support: window.len(),
            });
        }
    }

    Ok(patterns)
}

/// Extract transformation signal patterns
fn extract_transformation_patterns(segments: &[Segment]) -> Result<Vec<ContextPattern>> {
    let mut patterns = Vec::new();

    // Group segments by transformation count
    let low_transform_segments: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.transformations_count <= 2)
        .collect();

    let high_transform_segments: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.transformations_count >= 5)
        .collect();

    // Analyze common words in each group
    if !low_transform_segments.is_empty() {
        let common_words = find_common_words(&low_transform_segments, 5);
        patterns.push(ContextPattern {
            pattern_type: PatternType::TransformationSignal {
                low_transformations: true,
                context_type: infer_context_type(&common_words),
            },
            description: format!(
                "Low transformations ({} segments) associated with: {}",
                low_transform_segments.len(),
                common_words.join(", ")
            ),
            confidence: 0.85,
            support: low_transform_segments.len(),
        });
    }

    if !high_transform_segments.is_empty() {
        let common_words = find_common_words(&high_transform_segments, 5);
        patterns.push(ContextPattern {
            pattern_type: PatternType::TransformationSignal {
                low_transformations: false,
                context_type: infer_context_type(&common_words),
            },
            description: format!(
                "High transformations ({} segments) associated with: {}",
                high_transform_segments.len(),
                common_words.join(", ")
            ),
            confidence: 0.85,
            support: high_transform_segments.len(),
        });
    }

    Ok(patterns)
}

/// Find most common words in a set of segments
fn find_common_words(segments: &[&Segment], top_n: usize) -> Vec<String> {
    let mut word_freq: HashMap<String, usize> = HashMap::new();

    for segment in segments {
        for word in segment.text.split_whitespace() {
            let word_lower = word.to_lowercase();
            *word_freq.entry(word_lower).or_insert(0) += 1;
        }
    }

    let mut words: Vec<(&String, &usize)> = word_freq.iter().collect();
    words.sort_by(|a, b| b.1.cmp(a.1));

    words
        .into_iter()
        .take(top_n)
        .map(|(word, _)| word.clone())
        .collect()
}

/// Infer context type from common words
fn infer_context_type(words: &[String]) -> String {
    let technical_words = ["class", "function", "method", "api", "code"];
    let email_words = ["email", "regards", "attached", "please"];

    let tech_score = words.iter().filter(|w| technical_words.contains(&w.as_str())).count();
    let email_score = words.iter().filter(|w| email_words.contains(&w.as_str())).count();

    if tech_score > email_score {
        "Technical".to_string()
    } else if email_score > 0 {
        "Email".to_string()
    } else {
        "General".to_string()
    }
}
