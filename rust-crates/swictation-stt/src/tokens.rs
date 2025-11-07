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
    /// Format: Each line is a token, line number is the token ID
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| SttError::model_load(format!("Failed to open tokens file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut tokens = Vec::new();
        let mut token_to_id = HashMap::new();

        for (id, line) in reader.lines().enumerate() {
            let token = line.map_err(|e| {
                SttError::model_load(format!("Failed to read token line {}: {}", id, e))
            })?;

            token_to_id.insert(token.clone(), id);
            tokens.push(token);
        }

        if tokens.is_empty() {
            return Err(SttError::model_load("Token file is empty"));
        }

        // Blank token is typically ID 0 for RNN-T models
        let blank_id = 0;

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

            // Append token
            text.push_str(token);
            prev_id = id;
        }

        Ok(text)
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
