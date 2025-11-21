//! Topic clustering using vocabulary similarity

use crate::Segment;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A discovered topic cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCluster {
    pub id: usize,
    pub name: String,
    pub keywords: Vec<String>,
    pub segment_count: usize,
    pub confidence: f64,
}

/// Discover topic clusters from segments
pub fn discover_topics(segments: &[Segment], num_clusters: usize) -> Result<Vec<TopicCluster>> {
    // Build vocabulary frequency map
    let mut word_freq: HashMap<String, usize> = HashMap::new();

    for segment in segments {
        for word in segment.text.split_whitespace() {
            let word_lower = word.to_lowercase();
            *word_freq.entry(word_lower).or_insert(0) += 1;
        }
    }

    // Simple clustering: group by most frequent words
    // In production, would use k-means on TF-IDF vectors
    let mut top_words: Vec<(&String, &usize)> = word_freq.iter().collect();
    top_words.sort_by(|a, b| b.1.cmp(a.1));

    let words_per_cluster = top_words.len() / num_clusters.max(1);

    let mut clusters = Vec::new();
    for i in 0..num_clusters {
        let start = i * words_per_cluster;
        let end = ((i + 1) * words_per_cluster).min(top_words.len());

        if start >= top_words.len() {
            break;
        }

        let keywords: Vec<String> = top_words[start..end]
            .iter()
            .take(10)
            .map(|(word, _)| word.to_string())
            .collect();

        // Infer cluster name from keywords
        let name = infer_cluster_name(&keywords);

        clusters.push(TopicCluster {
            id: i,
            name,
            keywords,
            segment_count: 0, // Will be populated later
            confidence: 0.8,
        });
    }

    // Count segments per cluster (assign to cluster with most keyword matches)
    for segment in segments {
        let segment_words: Vec<String> = segment
            .text
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        let mut best_cluster = 0;
        let mut max_matches = 0;

        for (idx, cluster) in clusters.iter().enumerate() {
            let matches = cluster
                .keywords
                .iter()
                .filter(|kw| segment_words.contains(kw))
                .count();

            if matches > max_matches {
                max_matches = matches;
                best_cluster = idx;
            }
        }

        if let Some(cluster) = clusters.get_mut(best_cluster) {
            cluster.segment_count += 1;
        }
    }

    Ok(clusters)
}

/// Infer human-readable cluster name from keywords
fn infer_cluster_name(keywords: &[String]) -> String {
    // Technical indicators
    let technical_words = [
        "class", "function", "method", "api", "code", "refactor",
        "database", "query", "endpoint", "authentication", "object",
    ];

    // Business indicators
    let business_words = [
        "meeting", "budget", "team", "project", "deadline", "manager",
        "client", "presentation", "report", "revenue",
    ];

    // Email indicators
    let email_words = [
        "email", "regards", "attached", "please", "thank", "sincerely",
        "cc", "bcc", "subject", "forward",
    ];

    let tech_score = keywords
        .iter()
        .filter(|kw| technical_words.contains(&kw.as_str()))
        .count();

    let business_score = keywords
        .iter()
        .filter(|kw| business_words.contains(&kw.as_str()))
        .count();

    let email_score = keywords
        .iter()
        .filter(|kw| email_words.contains(&kw.as_str()))
        .count();

    if tech_score > business_score && tech_score > email_score {
        "Software Development".to_string()
    } else if business_score > email_score {
        "Business/Meetings".to_string()
    } else if email_score > 0 {
        "Email/Communication".to_string()
    } else {
        format!("General (cluster {})", keywords.first().unwrap_or(&String::from("unknown")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_infer_cluster_name() {
        let tech_keywords = vec![
            "class".to_string(),
            "function".to_string(),
            "api".to_string(),
        ];
        assert_eq!(infer_cluster_name(&tech_keywords), "Software Development");

        let business_keywords = vec![
            "meeting".to_string(),
            "budget".to_string(),
            "team".to_string(),
        ];
        assert_eq!(infer_cluster_name(&business_keywords), "Business/Meetings");
    }

    #[test]
    fn test_discover_topics() {
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
                text: "meeting with the team about budget".to_string(),
                words: 6,
                transformations_count: 0,
            },
        ];

        let result = discover_topics(&segments, 2);
        assert!(result.is_ok());

        let clusters = result.unwrap();
        assert!(clusters.len() <= 2);
    }
}
