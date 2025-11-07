//! Token vocabulary and decoding for Parakeet model

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{Result, SttError};

/// Token decoder for converting model output to text
pub struct TokenDecoder {
    /// Token ID to string mapping
    tokens: Vec<String>,
    /// String to token ID mapping
    token_to_id: HashMap<String, usize>,
    /// Blank token ID (for CTC/RNN-T)
    blank_id: usize,
}

impl TokenDecoder {
    /// Load tokens from tokens.txt file
    ///
    /// Format: Each line is "token_text token_id"
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| SttError::model_load(format!("Failed to open tokens file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut tokens = Vec::new();
        let mut token_to_id = HashMap::new();
        let mut max_id = 0;

        // First pass: determine max ID to size the vector
        for line in reader.lines() {
            let line = line.map_err(|e| {
                SttError::model_load(format!("Failed to read token line: {}", e))
            })?;

            // Parse line format: "token_text token_id"
            // Split from the right to handle tokens that contain spaces
            if let Some(last_space_idx) = line.rfind(' ') {
                let id_str = &line[last_space_idx + 1..];
                if let Ok(id) = id_str.parse::<usize>() {
                    max_id = max_id.max(id);
                }
            }
        }

        // Initialize tokens vector with correct size
        tokens.resize(max_id + 1, String::new());

        // Second pass: populate the tokens
        let file = File::open(path.as_ref())
            .map_err(|e| SttError::model_load(format!("Failed to open tokens file: {}", e)))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| {
                SttError::model_load(format!("Failed to read token line: {}", e))
            })?;

            // Parse line format: "token_text token_id"
            if let Some(last_space_idx) = line.rfind(' ') {
                let token_text = &line[..last_space_idx];
                let id_str = &line[last_space_idx + 1..];

                if let Ok(id) = id_str.parse::<usize>() {
                    tokens[id] = token_text.to_string();
                    token_to_id.insert(token_text.to_string(), id);
                }
            }
        }

        if tokens.is_empty() {
            return Err(SttError::model_load("Token file is empty"));
        }

        // For Parakeet TDT, blank token is at the last position (vocab_size - 1)
        let blank_id = tokens.len() - 1;

        println!("Loaded {} tokens (blank_id: {})", tokens.len(), blank_id);

        Ok(Self {
            tokens,
            token_to_id,
            blank_id,
        })
    }

    /// Decode a sequence of token IDs to text
    pub fn decode(&self, token_ids: &[usize]) -> Result<String> {
        let mut text = String::new();
        let mut prev_id = self.blank_id;

        for &id in token_ids {
            // Skip blank tokens
            if id == self.blank_id {
                prev_id = id;
                continue;
            }

            // Skip repeated tokens (RNN-T collapse)
            if id == prev_id {
                continue;
            }

            // Get token string
            let token = self.tokens.get(id).ok_or_else(|| {
                SttError::TokenDecodingError(format!("Invalid token ID: {}", id))
            })?;

            // Skip special tokens (those starting with <)
            if token.starts_with('<') {
                prev_id = id;
                continue;
            }

            // Append token
            text.push_str(token);
            prev_id = id;
        }

        // Post-process: Convert sentencepiece format to readable text
        // Replace ▁ (U+2581 LOWER ONE EIGHTH BLOCK) with space
        let text = text.replace('▁', " ");

        // Trim leading/trailing whitespace
        Ok(text.trim().to_string())
    }

    /// Get token ID for a string
    pub fn encode(&self, token: &str) -> Option<usize> {
        self.token_to_id.get(token).copied()
    }

    /// Get blank token ID
    pub fn blank_id(&self) -> usize {
        self.blank_id
    }

    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }

    /// Get token string by ID
    pub fn id_to_token(&self, id: usize) -> Option<&str> {
        self.tokens.get(id).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_decoder_basic() {
        // Create a simple test tokens file
        let tokens = vec![
            "<blk>".to_string(),
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            " ".to_string(),
        ];

        let decoder = TokenDecoder {
            tokens: tokens.clone(),
            token_to_id: tokens
                .iter()
                .enumerate()
                .map(|(i, t)| (t.clone(), i))
                .collect(),
            blank_id: 0,
        };

        // Test decoding: [1, 1, 0, 2, 4, 3] -> "ab c"
        let token_ids = vec![1, 1, 0, 2, 4, 3];
        let text = decoder.decode(&token_ids).unwrap();
        assert_eq!(text, "ab c");
    }

    #[test]
    fn test_blank_tokens() {
        let decoder = TokenDecoder {
            tokens: vec!["<blk>".to_string(), "x".to_string(), "y".to_string()],
            token_to_id: HashMap::new(),
            blank_id: 0,
        };

        // [0, 1, 0, 0, 2, 0] -> "xy"
        let text = decoder.decode(&[0, 1, 0, 0, 2, 0]).unwrap();
        assert_eq!(text, "xy");
    }
}
