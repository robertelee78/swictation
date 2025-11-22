//! Learned Pattern Corrections Engine
//!
//! Provides hot-reloadable user corrections that apply after midstream's
//! static transform rules. Supports exact and phonetic matching.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// A single learned correction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub id: String,
    pub original: String,
    pub corrected: String,
    pub mode: CorrectionMode,
    pub match_type: MatchType,
    #[serde(default = "default_case_mode")]
    pub case_mode: CaseMode,
    pub learned_at: DateTime<Utc>,
    pub use_count: u64,
}

fn default_case_mode() -> CaseMode {
    CaseMode::PreserveInput
}

/// Which transformation mode(s) this correction applies to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionMode {
    Secretary,
    Code,
    All,
}

impl CorrectionMode {
    pub fn matches(&self, current_mode: &str) -> bool {
        match self {
            CorrectionMode::All => true,
            CorrectionMode::Secretary => current_mode.eq_ignore_ascii_case("secretary"),
            CorrectionMode::Code => current_mode.eq_ignore_ascii_case("code"),
        }
    }
}

/// How to match the original text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    Exact,
    Phonetic,
}

/// How to handle case when applying corrections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    /// Match output case to input case (default)
    PreserveInput,
    /// Always use correction's case regardless of input
    ForcePattern,
    /// Use correction case unless input is all-caps
    Smart,
}

/// TOML file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CorrectionsFile {
    #[serde(default)]
    corrections: Vec<Correction>,
}

/// The correction engine with hot-reloading support
pub struct CorrectionEngine {
    /// Path to corrections.toml
    config_path: PathBuf,

    /// Exact phrase matches (multi-word), keyed by lowercase original
    exact_phrases: Arc<RwLock<HashMap<String, Correction>>>,

    /// Exact word matches (single word), keyed by lowercase original
    exact_words: Arc<RwLock<HashMap<String, Correction>>>,

    /// Phonetic phrase matches, sorted longest-first
    phonetic_phrases: Arc<RwLock<Vec<Correction>>>,

    /// Phonetic word matches
    phonetic_words: Arc<RwLock<Vec<Correction>>>,

    /// Phonetic similarity threshold (0.0 to 1.0, lower = more strict)
    phonetic_threshold: f64,

    /// In-memory use count tracking (correction ID → match count)
    use_counts: Arc<RwLock<HashMap<String, u64>>>,

    /// Total matches since last flush (for batching)
    total_matches: Arc<RwLock<u64>>,

    /// File watcher handle
    _watcher: Option<RecommendedWatcher>,
}

impl CorrectionEngine {
    /// Create a new correction engine and load corrections from disk
    pub fn new(config_dir: PathBuf, phonetic_threshold: f64) -> Self {
        let config_path = config_dir.join("corrections.toml");

        let mut engine = Self {
            config_path,
            exact_phrases: Arc::new(RwLock::new(HashMap::new())),
            exact_words: Arc::new(RwLock::new(HashMap::new())),
            phonetic_phrases: Arc::new(RwLock::new(Vec::new())),
            phonetic_words: Arc::new(RwLock::new(Vec::new())),
            phonetic_threshold,
            use_counts: Arc::new(RwLock::new(HashMap::new())),
            total_matches: Arc::new(RwLock::new(0)),
            _watcher: None,
        };

        // Initial load
        if let Err(e) = engine.reload() {
            warn!("Failed to load corrections: {}", e);
        }

        engine
    }

    /// Start watching the config file for changes
    pub fn start_watching(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let exact_phrases = Arc::clone(&self.exact_phrases);
        let exact_words = Arc::clone(&self.exact_words);
        let phonetic_phrases = Arc::clone(&self.phonetic_phrases);
        let phonetic_words = Arc::clone(&self.phonetic_words);
        let config_path = self.config_path.clone();
        let threshold = self.phonetic_threshold;

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        info!("Corrections file changed, reloading...");
                        if let Err(e) = Self::reload_into(
                            &config_path,
                            &exact_phrases,
                            &exact_words,
                            &phonetic_phrases,
                            &phonetic_words,
                            threshold,
                        ) {
                            error!("Failed to reload corrections: {}", e);
                        }
                    }
                }
                Err(e) => error!("File watch error: {}", e),
            })?;

        // Watch the config directory (not just the file, in case it's recreated)
        if let Some(parent) = self.config_path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

        self._watcher = Some(watcher);
        info!("Watching {:?} for changes", self.config_path);
        Ok(())
    }

    /// Reload corrections from disk
    pub fn reload(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::reload_into(
            &self.config_path,
            &self.exact_phrases,
            &self.exact_words,
            &self.phonetic_phrases,
            &self.phonetic_words,
            self.phonetic_threshold,
        )
    }

    fn reload_into(
        config_path: &PathBuf,
        exact_phrases: &Arc<RwLock<HashMap<String, Correction>>>,
        exact_words: &Arc<RwLock<HashMap<String, Correction>>>,
        phonetic_phrases: &Arc<RwLock<Vec<Correction>>>,
        phonetic_words: &Arc<RwLock<Vec<Correction>>>,
        _threshold: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = match fs::read_to_string(config_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!(
                    "No corrections file found at {:?}, starting fresh",
                    config_path
                );
                return Ok(());
            }
            Err(e) => return Err(Box::new(e)),
        };

        let file: CorrectionsFile = toml::from_str(&content)?;

        // Separate into categories
        let mut new_exact_phrases = HashMap::new();
        let mut new_exact_words = HashMap::new();
        let mut new_phonetic_phrases = Vec::new();
        let mut new_phonetic_words = Vec::new();

        for correction in file.corrections {
            let key = correction.original.to_lowercase();
            let is_phrase = key.contains(' ');

            match correction.match_type {
                MatchType::Exact => {
                    if is_phrase {
                        new_exact_phrases.insert(key, correction);
                    } else {
                        new_exact_words.insert(key, correction);
                    }
                }
                MatchType::Phonetic => {
                    if is_phrase {
                        new_phonetic_phrases.push(correction);
                    } else {
                        new_phonetic_words.push(correction);
                    }
                }
            }
        }

        // Sort phonetic patterns by length (longest first)
        new_phonetic_phrases.sort_by(|a, b| {
            b.original
                .split_whitespace()
                .count()
                .cmp(&a.original.split_whitespace().count())
        });
        new_phonetic_words.sort_by(|a, b| b.original.len().cmp(&a.original.len()));

        // Swap in new data
        *exact_phrases.write().unwrap() = new_exact_phrases;
        *exact_words.write().unwrap() = new_exact_words;
        *phonetic_phrases.write().unwrap() = new_phonetic_phrases;
        *phonetic_words.write().unwrap() = new_phonetic_words;

        info!(
            "Loaded corrections: {} exact phrases, {} exact words, {} phonetic phrases, {} phonetic words",
            exact_phrases.read().unwrap().len(),
            exact_words.read().unwrap().len(),
            phonetic_phrases.read().unwrap().len(),
            phonetic_words.read().unwrap().len(),
        );

        Ok(())
    }

    /// Apply learned corrections to text
    ///
    /// Matching order:
    /// 1. Exact phrase matches (longest first)
    /// 2. Exact word matches
    /// 3. Phonetic phrase matches (longest first)
    /// 4. Phonetic word matches
    pub fn apply(&self, text: &str, mode: &str) -> String {
        let start = Instant::now();

        // Pre-allocate result
        let mut result = String::with_capacity(text.len() + 32);

        // Tokenize once, lowercase once
        let words: Vec<&str> = text.split_whitespace().collect();
        let words_lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

        let exact_phrases = self.exact_phrases.read().unwrap();
        let exact_words = self.exact_words.read().unwrap();
        let phonetic_phrases = self.phonetic_phrases.read().unwrap();
        let phonetic_words = self.phonetic_words.read().unwrap();

        // Reusable key buffer for phrase matching
        let mut key_buf = String::with_capacity(64);

        let mut i = 0;
        while i < words.len() {
            let mut matched = false;

            // Try exact phrase matches (4-word, 3-word, 2-word)
            for phrase_len in (2..=4).rev() {
                if i + phrase_len <= words.len() {
                    key_buf.clear();
                    for j in 0..phrase_len {
                        if j > 0 {
                            key_buf.push(' ');
                        }
                        key_buf.push_str(&words_lower[i + j]);
                    }

                    if let Some(correction) = exact_phrases.get(&key_buf) {
                        if correction.mode.matches(mode) {
                            if !result.is_empty() {
                                result.push(' ');
                            }
                            // Apply case mode to replacement
                            let replacement = Self::preserve_case(
                                words[i],
                                &correction.corrected,
                                correction.case_mode,
                            );
                            result.push_str(&replacement);

                            // Track usage
                            self.increment_usage(&correction.id);

                            i += phrase_len;
                            matched = true;
                            break;
                        }
                    }
                }
            }

            if matched {
                continue;
            }

            // Try exact word match
            if let Some(correction) = exact_words.get(&words_lower[i]) {
                if correction.mode.matches(mode) {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    let replacement =
                        Self::preserve_case(words[i], &correction.corrected, correction.case_mode);
                    result.push_str(&replacement);

                    // Track usage
                    self.increment_usage(&correction.id);

                    i += 1;
                    continue;
                }
            }

            // Try phonetic phrase matches (longest first)
            for correction in phonetic_phrases.iter() {
                if !correction.mode.matches(mode) {
                    continue;
                }

                let pattern_words: Vec<&str> = correction.original.split_whitespace().collect();
                let pattern_len = pattern_words.len();

                if i + pattern_len <= words.len() {
                    // Build phrase from input
                    key_buf.clear();
                    for j in 0..pattern_len {
                        if j > 0 {
                            key_buf.push(' ');
                        }
                        key_buf.push_str(&words_lower[i + j]);
                    }

                    let distance = Self::normalized_edit_distance(
                        &key_buf,
                        &correction.original.to_lowercase(),
                    );
                    if distance <= self.phonetic_threshold {
                        if !result.is_empty() {
                            result.push(' ');
                        }
                        let replacement = Self::preserve_case(
                            words[i],
                            &correction.corrected,
                            correction.case_mode,
                        );
                        result.push_str(&replacement);

                        // Track usage
                        self.increment_usage(&correction.id);

                        i += pattern_len;
                        matched = true;
                        break;
                    }
                }
            }

            if matched {
                continue;
            }

            // Try phonetic word match
            for correction in phonetic_words.iter() {
                if !correction.mode.matches(mode) {
                    continue;
                }

                let distance = Self::normalized_edit_distance(
                    &words_lower[i],
                    &correction.original.to_lowercase(),
                );
                if distance <= self.phonetic_threshold {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    let replacement =
                        Self::preserve_case(words[i], &correction.corrected, correction.case_mode);
                    result.push_str(&replacement);

                    // Track usage
                    self.increment_usage(&correction.id);

                    matched = true;
                    break;
                }
            }

            if matched {
                i += 1;
                continue;
            }

            // No match - keep original word
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(words[i]);
            i += 1;
        }

        let elapsed = start.elapsed();
        debug!("Corrections applied in {:?}", elapsed);

        result
    }

    /// Preserve the case pattern of the original word in the replacement
    fn preserve_case(original: &str, replacement: &str, case_mode: CaseMode) -> String {
        if original.is_empty() || replacement.is_empty() {
            return replacement.to_string();
        }

        match case_mode {
            CaseMode::ForcePattern => {
                // Always use the correction's case exactly as specified
                replacement.to_string()
            }
            CaseMode::Smart => {
                // Use correction case unless input is all-caps
                let all_upper = original
                    .chars()
                    .all(|c| c.is_uppercase() || !c.is_alphabetic());

                if all_upper && original.len() > 1 {
                    // Input is ALL CAPS -> make output all caps
                    replacement.to_uppercase()
                } else {
                    // Otherwise use correction's case
                    replacement.to_string()
                }
            }
            CaseMode::PreserveInput => {
                // Match output case to input case (original behavior)
                let first_char = original.chars().next().unwrap();
                let all_upper = original
                    .chars()
                    .all(|c| c.is_uppercase() || !c.is_alphabetic());

                if all_upper && original.len() > 1 {
                    // ALL CAPS -> ALL CAPS
                    replacement.to_uppercase()
                } else if first_char.is_uppercase() {
                    // Title Case -> Title Case
                    let mut chars = replacement.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                } else {
                    // lowercase -> lowercase
                    replacement.to_lowercase()
                }
            }
        }
    }

    /// Compute normalized Levenshtein edit distance (0.0 = identical, 1.0 = completely different)
    fn normalized_edit_distance(a: &str, b: &str) -> f64 {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let n = a_chars.len();
        let m = b_chars.len();

        if n == 0 {
            return if m == 0 { 0.0 } else { 1.0 };
        }
        if m == 0 {
            return 1.0;
        }

        // Use single-row optimization for memory efficiency
        let mut prev_row: Vec<usize> = (0..=m).collect();
        let mut curr_row: Vec<usize> = vec![0; m + 1];

        for i in 1..=n {
            curr_row[0] = i;

            for j in 1..=m {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                curr_row[j] = (prev_row[j] + 1)
                    .min(curr_row[j - 1] + 1)
                    .min(prev_row[j - 1] + cost);
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        let distance = prev_row[m];
        distance as f64 / n.max(m) as f64
    }

    /// Add a new correction and save to disk
    pub fn learn(
        &self,
        original: String,
        corrected: String,
        mode: CorrectionMode,
        match_type: MatchType,
    ) -> Result<Correction, Box<dyn std::error::Error + Send + Sync>> {
        let correction = Correction {
            id: Uuid::new_v4().to_string(),
            original: original.to_lowercase(),
            corrected,
            mode,
            match_type,
            case_mode: CaseMode::PreserveInput,
            learned_at: Utc::now(),
            use_count: 0,
        };

        // Load existing, add new, save
        let mut file = self.load_file()?;

        // Check for duplicate (same original + mode)
        file.corrections.retain(|c| {
            !(c.original.to_lowercase() == correction.original.to_lowercase()
                && c.mode == correction.mode)
        });

        file.corrections.push(correction.clone());
        self.save_file(&file)?;

        info!(
            "Learned correction: '{}' -> '{}'",
            correction.original, correction.corrected
        );
        Ok(correction)
    }

    /// Get all corrections
    pub fn get_all(&self) -> Result<Vec<Correction>, Box<dyn std::error::Error + Send + Sync>> {
        let file = self.load_file()?;
        Ok(file.corrections)
    }

    /// Delete a correction by ID
    pub fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut file = self.load_file()?;
        let original_len = file.corrections.len();
        file.corrections.retain(|c| c.id != id);

        if file.corrections.len() == original_len {
            return Err("Correction not found".into());
        }

        self.save_file(&file)?;
        info!("Deleted correction: {}", id);
        Ok(())
    }

    /// Update a correction
    pub fn update(
        &self,
        id: &str,
        corrected: Option<String>,
        mode: Option<CorrectionMode>,
        match_type: Option<MatchType>,
    ) -> Result<Correction, Box<dyn std::error::Error + Send + Sync>> {
        let mut file = self.load_file()?;

        let correction = file
            .corrections
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("Correction not found")?;

        if let Some(c) = corrected {
            correction.corrected = c;
        }
        if let Some(m) = mode {
            correction.mode = m;
        }
        if let Some(mt) = match_type {
            correction.match_type = mt;
        }

        let updated = correction.clone();
        self.save_file(&file)?;

        info!("Updated correction: {}", id);
        Ok(updated)
    }

    /// Increment usage count for a correction (in-memory only)
    fn increment_usage(&self, correction_id: &str) {
        let mut counts = self.use_counts.write().unwrap();
        *counts.entry(correction_id.to_string()).or_insert(0) += 1;

        // Track total matches for batching
        let mut total = self.total_matches.write().unwrap();
        *total += 1;
    }

    /// Flush usage counts to disk (batched write)
    pub fn flush_usage_counts(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let counts = self.use_counts.read().unwrap();

        if counts.is_empty() {
            return Ok(()); // Nothing to flush
        }

        let mut file = self.load_file()?;

        // Update use_count for each correction
        for correction in &mut file.corrections {
            if let Some(&count) = counts.get(&correction.id) {
                correction.use_count += count;
            }
        }

        self.save_file(&file)?;

        // Clear in-memory counts after successful flush
        drop(counts);
        self.use_counts.write().unwrap().clear();
        *self.total_matches.write().unwrap() = 0;

        info!("Flushed usage counts to disk");
        Ok(())
    }

    /// Check if we should flush based on match count
    pub fn should_flush(&self) -> bool {
        let total = self.total_matches.read().unwrap();
        *total >= 50 // Flush after 50 matches
    }

    fn load_file(&self) -> Result<CorrectionsFile, Box<dyn std::error::Error + Send + Sync>> {
        match fs::read_to_string(&self.config_path) {
            Ok(content) => Ok(toml::from_str(&content)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CorrectionsFile::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn save_file(
        &self,
        file: &CorrectionsFile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ensure directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(file)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance() {
        assert_eq!(
            CorrectionEngine::normalized_edit_distance("kitten", "sitting"),
            3.0 / 7.0
        );
        assert_eq!(
            CorrectionEngine::normalized_edit_distance("hello", "hello"),
            0.0
        );
        assert_eq!(CorrectionEngine::normalized_edit_distance("", ""), 0.0);
        assert_eq!(CorrectionEngine::normalized_edit_distance("abc", ""), 1.0);
    }

    #[test]
    fn test_preserve_case() {
        assert_eq!(
            CorrectionEngine::preserve_case("Hello", "world", CaseMode::PreserveInput),
            "World"
        );
        assert_eq!(
            CorrectionEngine::preserve_case("HELLO", "world", CaseMode::PreserveInput),
            "WORLD"
        );
        assert_eq!(
            CorrectionEngine::preserve_case("hello", "World", CaseMode::PreserveInput),
            "world"
        );
    }

    #[test]
    fn test_phonetic_threshold() {
        // "arkon" vs "archon" - edit distance = 2 (delete 'k', add 'ch')
        // Actually: a-r-k-o-n vs a-r-c-h-o-n
        // Levenshtein: 2 edits (k->c, insert h)
        // Normalized: 2/6 = 0.333
        let dist = CorrectionEngine::normalized_edit_distance("arkon", "archon");
        assert!(dist > 0.3, "Expected >0.3, got {}", dist);
        assert!(dist < 0.4, "Expected <0.4, got {}", dist);
    }
}
