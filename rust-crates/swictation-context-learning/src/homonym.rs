//! Homonym resolution using context patterns

use crate::{Segment, TopicCluster};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Homonym resolution rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomonymResolver {
    pub word: String,
    pub interpretations: Vec<Interpretation>,
}

/// A possible interpretation of a homonym
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interpretation {
    pub meaning: String,
    pub context_keywords: Vec<String>,
    pub confidence: f64,
    pub frequency: usize,
}

/// Learn homonym resolution rules from segments
pub fn learn_homonym_rules(
    segments: &[Segment],
    topics: &[TopicCluster],
) -> Result<HashMap<String, HomonymResolver>> {
    let mut rules = HashMap::new();

    // Common homonyms to analyze
    let homonyms = [
        "class", "object", "method", "read", "write", "to", "too", "two", "their", "there",
        "theyre", "your", "youre",
    ];

    for homonym in &homonyms {
        let resolver = analyze_homonym(homonym, segments, topics)?;
        if !resolver.interpretations.is_empty() {
            rules.insert(homonym.to_string(), resolver);
        }
    }

    Ok(rules)
}

/// Analyze a specific homonym across all segments
fn analyze_homonym(
    word: &str,
    segments: &[Segment],
    topics: &[TopicCluster],
) -> Result<HomonymResolver> {
    let mut interpretations: HashMap<String, Interpretation> = HashMap::new();

    // Find segments containing this word
    for segment in segments {
        let segment_words: Vec<String> = segment
            .text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        if !segment_words.contains(&word.to_string()) {
            continue;
        }

        // Determine which topic this segment belongs to
        let topic = find_segment_topic(segment, topics);

        // Create interpretation based on topic
        let interpretation_key = topic
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "General".to_string());

        interpretations
            .entry(interpretation_key.clone())
            .and_modify(|interp| {
                interp.frequency += 1;
            })
            .or_insert_with(|| Interpretation {
                meaning: format!("{} in {} context", word, interpretation_key),
                context_keywords: topic.map(|t| t.keywords.clone()).unwrap_or_default(),
                confidence: 0.0, // Will be calculated later
                frequency: 1,
            });
    }

    // Calculate confidence scores
    let total_freq: usize = interpretations.values().map(|i| i.frequency).sum();

    for interp in interpretations.values_mut() {
        interp.confidence = if total_freq > 0 {
            interp.frequency as f64 / total_freq as f64
        } else {
            0.0
        };
    }

    // Sort by frequency
    let mut interp_list: Vec<Interpretation> = interpretations.into_values().collect();
    interp_list.sort_by(|a, b| b.frequency.cmp(&a.frequency));

    Ok(HomonymResolver {
        word: word.to_string(),
        interpretations: interp_list,
    })
}

/// Find which topic cluster a segment belongs to
fn find_segment_topic<'a>(
    segment: &Segment,
    topics: &'a [TopicCluster],
) -> Option<&'a TopicCluster> {
    let segment_words: Vec<String> = segment
        .text
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();

    let mut best_topic: Option<&TopicCluster> = None;
    let mut max_matches = 0;

    for topic in topics {
        let matches = topic
            .keywords
            .iter()
            .filter(|kw| segment_words.contains(kw))
            .count();

        if matches > max_matches {
            max_matches = matches;
            best_topic = Some(topic);
        }
    }

    best_topic
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_analyze_homonym() {
        let segments = vec![
            Segment {
                segment_id: 1,
                session_id: 1,
                timestamp: Utc::now(),
                text: "refactor the authentication class".to_string(),
                words: 4,
                transformations_count: 0,
            },
            Segment {
                segment_id: 2,
                session_id: 1,
                timestamp: Utc::now(),
                text: "upper class society".to_string(),
                words: 3,
                transformations_count: 0,
            },
        ];

        let topics = vec![
            TopicCluster {
                id: 0,
                name: "Software Development".to_string(),
                keywords: vec!["refactor".to_string(), "authentication".to_string()],
                segment_count: 1,
                confidence: 0.9,
            },
            TopicCluster {
                id: 1,
                name: "General".to_string(),
                keywords: vec!["upper".to_string(), "society".to_string()],
                segment_count: 1,
                confidence: 0.8,
            },
        ];

        let resolver = analyze_homonym("class", &segments, &topics).unwrap();
        assert_eq!(resolver.word, "class");
        assert_eq!(resolver.interpretations.len(), 2);
    }
}
