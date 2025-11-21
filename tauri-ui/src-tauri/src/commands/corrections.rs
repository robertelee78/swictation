//! Tauri commands for learned pattern corrections
//!
//! These commands manage user corrections stored in ~/.config/swictation/corrections.toml

use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;
use uuid::Uuid;

/// A single learned correction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub id: String,
    pub original: String,
    pub corrected: String,
    pub mode: String,        // "secretary" | "code" | "all"
    pub match_type: String,  // "exact" | "phonetic"
    #[serde(default = "default_case_mode")]
    pub case_mode: String,   // "preserve_input" | "force_pattern" | "smart"
    pub learned_at: DateTime<Utc>,
    pub use_count: u64,
}

fn default_case_mode() -> String {
    "preserve_input".to_string()
}

/// TOML file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CorrectionsFile {
    #[serde(default)]
    corrections: Vec<Correction>,
}

/// State for corrections management
pub struct CorrectionsState {
    pub config_path: PathBuf,
}

impl CorrectionsState {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("swictation");

        // Ensure directory exists
        let _ = fs::create_dir_all(&config_dir);

        Self {
            config_path: config_dir.join("corrections.toml"),
        }
    }

    fn load_file(&self) -> Result<CorrectionsFile, String> {
        match fs::read_to_string(&self.config_path) {
            Ok(content) => toml::from_str(&content)
                .map_err(|e| format!("Failed to parse corrections.toml: {}", e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CorrectionsFile::default()),
            Err(e) => Err(format!("Failed to read corrections.toml: {}", e)),
        }
    }

    fn save_file(&self, file: &CorrectionsFile) -> Result<(), String> {
        let content = toml::to_string_pretty(file)
            .map_err(|e| format!("Failed to serialize corrections: {}", e))?;
        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write corrections.toml: {}", e))
    }
}

/// Learn a new correction pattern
#[tauri::command]
pub async fn learn_correction(
    state: State<'_, Mutex<CorrectionsState>>,
    original: String,
    corrected: String,
    mode: String,
    match_type: String,
    case_mode: Option<String>,
) -> Result<Correction, String> {
    let state = state.lock().unwrap();

    // Validate inputs
    if original.trim().is_empty() {
        return Err("Original text cannot be empty".to_string());
    }
    if corrected.trim().is_empty() {
        return Err("Corrected text cannot be empty".to_string());
    }
    if !["secretary", "code", "all"].contains(&mode.as_str()) {
        return Err("Mode must be 'secretary', 'code', or 'all'".to_string());
    }
    if !["exact", "phonetic"].contains(&match_type.as_str()) {
        return Err("Match type must be 'exact' or 'phonetic'".to_string());
    }

    let case_mode = case_mode.unwrap_or_else(default_case_mode);
    if !["preserve_input", "force_pattern", "smart"].contains(&case_mode.as_str()) {
        return Err("Case mode must be 'preserve_input', 'force_pattern', or 'smart'".to_string());
    }

    let correction = Correction {
        id: Uuid::new_v4().to_string(),
        original: original.trim().to_lowercase(),
        corrected: corrected.trim().to_string(),
        mode,
        match_type,
        case_mode,
        learned_at: Utc::now(),
        use_count: 0,
    };

    let mut file = state.load_file()?;

    // Check for duplicate (same original + mode) and update if exists
    let existing_idx = file.corrections.iter().position(|c| {
        c.original.to_lowercase() == correction.original.to_lowercase() && c.mode == correction.mode
    });

    if let Some(idx) = existing_idx {
        file.corrections[idx].corrected = correction.corrected.clone();
        file.corrections[idx].match_type = correction.match_type.clone();
        let updated = file.corrections[idx].clone();
        state.save_file(&file)?;
        return Ok(updated);
    }

    file.corrections.push(correction.clone());
    state.save_file(&file)?;

    Ok(correction)
}

/// Get all corrections
#[tauri::command]
pub async fn get_corrections(
    state: State<'_, Mutex<CorrectionsState>>,
) -> Result<Vec<Correction>, String> {
    let state = state.lock().unwrap();
    let file = state.load_file()?;
    Ok(file.corrections)
}

/// Delete a correction by ID
#[tauri::command]
pub async fn delete_correction(
    state: State<'_, Mutex<CorrectionsState>>,
    id: String,
) -> Result<(), String> {
    let state = state.lock().unwrap();
    let mut file = state.load_file()?;

    let original_len = file.corrections.len();
    file.corrections.retain(|c| c.id != id);

    if file.corrections.len() == original_len {
        return Err("Correction not found".to_string());
    }

    state.save_file(&file)
}

/// Update a correction
#[tauri::command]
pub async fn update_correction(
    state: State<'_, Mutex<CorrectionsState>>,
    id: String,
    original: Option<String>,
    corrected: Option<String>,
    mode: Option<String>,
    match_type: Option<String>,
    case_mode: Option<String>,
) -> Result<Correction, String> {
    let state = state.lock().unwrap();
    let mut file = state.load_file()?;

    let correction = file.corrections.iter_mut()
        .find(|c| c.id == id)
        .ok_or("Correction not found")?;

    if let Some(o) = original {
        if o.trim().is_empty() {
            return Err("Original text cannot be empty".to_string());
        }
        correction.original = o.trim().to_lowercase();
    }
    if let Some(c) = corrected {
        if c.trim().is_empty() {
            return Err("Corrected text cannot be empty".to_string());
        }
        correction.corrected = c.trim().to_string();
    }
    if let Some(m) = mode {
        if !["secretary", "code", "all"].contains(&m.as_str()) {
            return Err("Mode must be 'secretary', 'code', or 'all'".to_string());
        }
        correction.mode = m;
    }
    if let Some(mt) = match_type {
        if !["exact", "phonetic"].contains(&mt.as_str()) {
            return Err("Match type must be 'exact' or 'phonetic'".to_string());
        }
        correction.match_type = mt;
    }
    if let Some(cm) = case_mode {
        if !["preserve_input", "force_pattern", "smart"].contains(&cm.as_str()) {
            return Err("Case mode must be 'preserve_input', 'force_pattern', or 'smart'".to_string());
        }
        correction.case_mode = cm;
    }

    let updated = correction.clone();
    state.save_file(&file)?;

    Ok(updated)
}

/// Extract diff between original and edited text, returning pairs of (original_word, corrected_word)
#[tauri::command]
pub async fn extract_corrections_diff(
    original: String,
    edited: String,
) -> Result<Vec<(String, String)>, String> {
    let original_words: Vec<&str> = original.split_whitespace().collect();
    let edited_words: Vec<&str> = edited.split_whitespace().collect();

    // Simple LCS-based diff (works for most cases)
    let mut pairs = Vec::new();
    let lcs = longest_common_subsequence(&original_words, &edited_words);

    let mut orig_idx = 0;
    let mut edit_idx = 0;
    let mut lcs_idx = 0;

    while orig_idx < original_words.len() && edit_idx < edited_words.len() {
        if lcs_idx < lcs.len()
            && original_words[orig_idx] == lcs[lcs_idx]
            && edited_words[edit_idx] == lcs[lcs_idx]
        {
            // Match - both in LCS, skip
            orig_idx += 1;
            edit_idx += 1;
            lcs_idx += 1;
        } else if lcs_idx < lcs.len() && original_words[orig_idx] == lcs[lcs_idx] {
            // Insertion in edited - skip
            edit_idx += 1;
        } else if lcs_idx < lcs.len() && edited_words[edit_idx] == lcs[lcs_idx] {
            // Deletion from original - skip
            orig_idx += 1;
        } else {
            // Substitution - this is a correction!
            pairs.push((
                original_words[orig_idx].to_lowercase(),
                edited_words[edit_idx].to_string(),
            ));
            orig_idx += 1;
            edit_idx += 1;
        }
    }

    Ok(pairs)
}

/// Compute longest common subsequence
fn longest_common_subsequence<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let n = a.len();
    let m = b.len();

    // DP table
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to get LCS
    let mut result = Vec::new();
    let mut i = n;
    let mut j = m;

    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}
