use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use csm_core::{PatternRegistry, Pattern, DomainKind, CsmError, DomainTokenizer, SlotSchema};
use csm_tokenizer::{LogTokenizer, CodeTokenizer, TextTokenizer};

/// Builds pattern database from corpus using frequency-based discovery
pub struct PatternBuilder {
    domain: DomainKind,
    frequency_threshold: u32,
    min_pattern_len: usize,
    max_pattern_len: usize,
}

impl PatternBuilder {
    /// Create new pattern builder for domain
    pub fn new(domain: DomainKind) -> Result<Self, CsmError> {
        Ok(PatternBuilder {
            domain,
            frequency_threshold: 3,    // Minimum 3 occurrences
            min_pattern_len: 2,        // At least 2 tokens
            max_pattern_len: 5,        // At most 5 tokens
        })
    }
    
    /// Scan corpus file and build patterns
    pub fn build_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<PatternRegistry, CsmError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        // Collect token sequences and their frequencies
        let mut sequence_freqs: HashMap<Vec<String>, u32> = HashMap::new();
        
        for line in reader.lines() {
            let line = line?;
            
            // Tokenize based on domain
            let tokens = match self.domain {
                DomainKind::Log => {
                    let tokenizer = LogTokenizer::new(Default::default());
                    tokenizer
                        .tokenize(&line)
                        .iter()
                        .map(|t| t.raw.to_string())
                        .collect::<Vec<_>>()
                }
                DomainKind::Code => {
                    let tokenizer = CodeTokenizer::new(Default::default());
                    tokenizer
                        .tokenize(&line)
                        .iter()
                        .map(|t| t.raw.to_string())
                        .collect::<Vec<_>>()
                }
                DomainKind::Text => {
                    let tokenizer = TextTokenizer::new(Default::default());
                    tokenizer
                        .tokenize(&line)
                        .iter()
                        .map(|t| t.raw.to_string())
                        .collect::<Vec<_>>()
                }
                DomainKind::Generic => {
                    // Simple whitespace tokenization
                    line.split_whitespace().map(|s| s.to_string()).collect()
                }
            };
            
            // Generate n-grams (patterns)
            for window_size in self.min_pattern_len..=self.max_pattern_len.min(tokens.len()) {
                for window in tokens.windows(window_size) {
                    let sequence: Vec<String> = window.to_vec();
                    *sequence_freqs.entry(sequence).or_insert(0) += 1;
                }
            }
        }
        
        // Build patterns from frequent sequences
        let mut registry = PatternRegistry::new();
        let mut pattern_id = 0u32;
        
        // Sort by frequency (descending)
        let mut sorted_sequences: Vec<_> = sequence_freqs.into_iter().collect();
        sorted_sequences.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (sequence, freq) in sorted_sequences {
            // Filter by minimum frequency
            if freq < self.frequency_threshold {
                continue;
            }
            
            // Estimate compression gain (simplified)
            // gain = (token_count - 1) * frequency (saving 1 token per pattern match, repeated freq times)
            let compress_gain = ((sequence.len() as f32 - 1.0) * freq as f32).max(0.0);
            
            if compress_gain <= 0.0 {
                continue;
            }
            
            // Create pattern
            let pattern = Pattern {
                id: pattern_id,
                domain: self.domain,
                base_seq: Default::default(), // Would be filled with vocab IDs
                slot_schema: SlotSchema { slots: Default::default() },
                template: sequence.join(" "),
                freq,
                ppmi_score: 0.5,
                compress_gain,
                pgs_score: 0.7,
                stability: 0.8,
                final_score: compress_gain * 0.5,
                deprecated: false,
                _pad: [0; 3],
            };
            
            // Register pattern
            registry.register(pattern)?;
            pattern_id += 1;
            
            // Limit total patterns
            if pattern_id >= 1000 {
                break;
            }
        }
        
        // Freeze registry to prevent further modifications
        registry.freeze();
        
        println!("Built {} patterns from corpus (domain: {})", pattern_id, 
                 format!("{:?}", self.domain).to_lowercase());
        
        Ok(registry)
    }
    
    /// Set minimum pattern frequency threshold
    pub fn with_frequency_threshold(mut self, threshold: u32) -> Self {
        self.frequency_threshold = threshold;
        self
    }
    
    /// Set pattern length bounds
    pub fn with_pattern_length(mut self, min: usize, max: usize) -> Self {
        self.min_pattern_len = min;
        self.max_pattern_len = max;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_builder() {
        let result = PatternBuilder::new(DomainKind::Log);
        assert!(result.is_ok());
    }
}
