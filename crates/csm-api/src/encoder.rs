use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

use csm_core::{DomainKind, Vocab, PatternRegistry, CsmError, Token, SlotValue, Assignment, AssignmentKind, DomainTokenizer, SemanticRecord, FeatureVector, VocabId};
use csm_tokenizer::{LogTokenizer, CodeTokenizer, TextTokenizer};
use csm_encoding::{BitWriter, file_format::{CsmHeader, flags}};
use csm_core::fst_engine::FstEngine;
use csm_core::fallback::FallbackEngine;

#[derive(Clone)]
pub enum TokenizerEnum {
    Log(LogTokenizer),
    Code(CodeTokenizer),
    Text(TextTokenizer),
}

impl DomainTokenizer for TokenizerEnum {
    type Config = ();

    fn new(_config: Self::Config) -> Self {
        // For now, use default configs
        unimplemented!()
    }

    fn tokenize<'a>(&self, input: &'a str) -> Vec<Token<'a>> {
        match self {
            TokenizerEnum::Log(t) => t.tokenize(input),
            TokenizerEnum::Code(t) => t.tokenize(input),
            TokenizerEnum::Text(t) => t.tokenize(input),
        }
    }

    fn domain(&self) -> DomainKind {
        match self {
            TokenizerEnum::Log(_) => DomainKind::Log,
            TokenizerEnum::Code(_) => DomainKind::Code,
            TokenizerEnum::Text(_) => DomainKind::Text,
        }
    }
}

pub struct Encoder {
    domain: DomainKind,
    vocab: Vocab,
    patterns: PatternRegistry,
    tokenizer: TokenizerEnum,
    fst_engine: FstEngine,
    fallback_engine: FallbackEngine,
    bit_writer: BitWriter,
}

impl Encoder {
    pub fn new(domain: DomainKind, vocab: Vocab, patterns: PatternRegistry) -> Result<Self, CsmError> {
        let tokenizer = match domain {
            DomainKind::Log => TokenizerEnum::Log(LogTokenizer::new(Default::default())),
            DomainKind::Code => TokenizerEnum::Code(CodeTokenizer::new(Default::default())),
            DomainKind::Text => TokenizerEnum::Text(TextTokenizer::new(Default::default())),
            DomainKind::Generic => TokenizerEnum::Text(TextTokenizer::new(Default::default())), // fallback
        };

        let fst_engine = FstEngine::new();
        // TODO: build FST from patterns registry
        let fallback_engine = FallbackEngine::new();

        Ok(Encoder {
            domain,
            vocab,
            patterns,
            tokenizer,
            fst_engine,
            fallback_engine,
            bit_writer: BitWriter::new(),
        })
    }

    pub fn encode_file<P: AsRef<Path>>(&mut self, input_path: P, output_path: P) -> Result<(), CsmError> {
        let file = File::open(input_path)?;
        let reader = BufReader::new(file);
        
        let mut lines = Vec::new();
        for line in reader.lines() {
            let line = line?;
            lines.push(line);
        }
        
        self.encode_lines(&lines, output_path)
    }

    pub fn encode_lines<P: AsRef<Path>>(&mut self, lines: &[String], output_path: P) -> Result<(), CsmError> {
        // Tokenize all lines
        let mut tokenized_lines = Vec::new();
        for line in lines {
            let tokens = self.tokenizer.tokenize(line);
            let token_strings: Vec<String> = tokens.iter().map(|t| t.raw.to_string()).collect();
            tokenized_lines.push(token_strings);
        }
        
        // Build semantic records
        let mut semantic_records = Vec::new();
        for (line_idx, tokens) in tokenized_lines.iter().enumerate() {
            let record = self.build_semantic_record(line_idx, tokens)?;
            semantic_records.push(record);
        }
        
        // Run Viterbi DP to select optimal assignments
        let assignments = self.viterbi_select(&tokenized_lines)?;
        
        // Write to .csm file
        self.write_csm_file(output_path, &assignments, &semantic_records)
    }

    fn build_semantic_record(&self, line_idx: usize, tokens: &[String]) -> Result<SemanticRecord, CsmError> {
        // For now, create basic record
        Ok(SemanticRecord {
            source_id: format!("line_{}", line_idx),
            offset: line_idx as u64,
            line_number: Some(line_idx as u64),
            ingested_at: 0, // TODO
            log_timestamp: None,
            pattern_id: 0, // TODO
            template: tokens.join(" "),
            domain: self.domain,
            slots: Vec::new(), // TODO
            raw_token_count: tokens.len() as u16,
            encoded_bits: 0, // TODO
            compress_ratio: 1.0, // TODO
            features: None, // TODO: compute features
        })
    }

    fn viterbi_select(&self, tokenized_lines: &[Vec<String>]) -> Result<Vec<Assignment>, CsmError> {
        // Simplified Viterbi - for each line, try FST match, fallback to tokens
        let mut assignments = Vec::new();
        
        for (line_idx, tokens) in tokenized_lines.iter().enumerate() {
            // Convert strings to VocabIds (simplified - assume direct mapping)
            let vocab_ids: Vec<VocabId> = tokens.iter().enumerate().map(|(i, _)| i as VocabId).collect();
            
            if let Some(pattern_match) = self.fst_engine.query(&vocab_ids).first() {
                // Found pattern match
                let assignment = Assignment {
                    kind: AssignmentKind::Pattern { id: pattern_match.pattern_id },
                    start: pattern_match.start,
                    end: pattern_match.end,
                };
                assignments.push(assignment);
            } else {
                // Fallback to individual tokens
                for (i, &vocab_id) in vocab_ids.iter().enumerate() {
                    let assignment = Assignment {
                        kind: AssignmentKind::Token { id: vocab_id },
                        start: i,
                        end: i + 1,
                    };
                    assignments.push(assignment);
                }
            }
        }
        
        Ok(assignments)
    }

    fn write_csm_file<P: AsRef<Path>>(
        &mut self, 
        output_path: P, 
        assignments: &[Assignment], 
        semantic_records: &[SemanticRecord]
    ) -> Result<(), CsmError> {
        let mut file = File::create(output_path)?;
        
        // Create header
        let mut header = CsmHeader::default();
        header.domain = self.domain as u8;
        header.vocab_size = self.vocab.len() as u32;
        header.pattern_count = self.patterns.len() as u32;
        header.flags = flags::MULTI_TIER_PACK | flags::VITERBI_SELECT;
        header.token_count_original = assignments.len() as u64;
        // TODO: calculate encoded count and other stats
        
        // Write header
        let header_bytes = header.to_bytes();
        std::io::Write::write_all(&mut file, &header_bytes)?;
        
        // TODO: write vocab section, pattern section, etc.
        // For now, just write assignments using bit writer
        
        for assignment in assignments {
            self.encode_assignment(assignment)?;
        }
        
        // Write bit stream
        let bit_writer = std::mem::replace(&mut self.bit_writer, BitWriter::new());
        let bit_data = bit_writer.finish();
        std::io::Write::write_all(&mut file, &bit_data)?;
        
        Ok(())
    }

    fn encode_assignment(&mut self, assignment: &Assignment) -> Result<(), CsmError> {
        match &assignment.kind {
            AssignmentKind::Pattern { id } => {
                // 2-bit tier prefix for pattern (assume tier 0 for now)
                self.bit_writer.write_bits(0, 2);
                self.bit_writer.write_tiered(*id as u32, 0);
            }
            AssignmentKind::Token { id } => {
                // 2-bit tier prefix for token
                self.bit_writer.write_bits(1, 2);
                self.bit_writer.write_tiered(*id as u32, 0); // simplified tier
            }
            AssignmentKind::Fallback { level } => {
                // 2-bit tier prefix for fallback
                self.bit_writer.write_bits(3, 2);
                self.bit_writer.write_bits(*level as u32, 2);
            }
        }
        Ok(())
    }
}