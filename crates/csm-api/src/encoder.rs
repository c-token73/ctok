use std::fs::File;
use std::io::{BufReader, BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use csm_core::{DomainKind, Vocab, PatternRegistry, CsmError, Assignment, AssignmentKind, DomainTokenizer, SemanticRecord, VocabId, viterbi_select, Token};
use csm_tokenizer::{LogTokenizer, CodeTokenizer, TextTokenizer};
use csm_encoding::{BitWriter, file_format::{CsmHeader, flags, SectionWriter}, crc::crc32fast};
use csm_core::fst_engine::FstEngine;
use csm_core::fallback::FallbackEngine;

#[derive(Debug, Clone)]
struct LineMetadata {
    source_offset: u64,
    log_ts: i64,
    token_count_orig: u16,
}

impl LineMetadata {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.source_offset.to_le_bytes());
        bytes.extend_from_slice(&self.log_ts.to_le_bytes());
        bytes.extend_from_slice(&self.token_count_orig.to_le_bytes());
        bytes
    }
}

#[derive(Clone)]
pub enum TokenizerEnum {
    Log(LogTokenizer),
    Code(CodeTokenizer),
    Text(TextTokenizer),
}

impl DomainTokenizer for TokenizerEnum {
    type Config = ();

    fn new(_config: Self::Config) -> Self {
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
            DomainKind::Generic => TokenizerEnum::Text(TextTokenizer::new(Default::default())),
        };

        let fst_engine = FstEngine::from_registry(&patterns);
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
        // Tokenize all lines and map to vocab IDs
        let mut tokenized_ids: Vec<Vec<VocabId>> = Vec::new();
        let mut token_text_lines: Vec<Vec<String>> = Vec::new();

        for line in lines {
            let tokens = self.tokenizer.tokenize(line);
            let mut line_ids = Vec::new();
            let mut line_texts = Vec::new();

            for token in tokens {
                let token_str = token.raw.to_string();
                let token_id = self.vocab.insert(&token_str)?;
                line_ids.push(token_id);
                line_texts.push(token_str);
            }

            tokenized_ids.push(line_ids);
            token_text_lines.push(line_texts);
        }

        // Build semantic records (proto)
        let mut semantic_records = Vec::new();
        for (line_idx, token_texts) in token_text_lines.iter().enumerate() {
            let record = self.build_semantic_record(line_idx, token_texts)?;
            semantic_records.push(record);
        }

        // Viterbi selection per line using FST matches
        let mut assignments = Vec::new();
        for ids in &tokenized_ids {
            let matches = self.fst_engine.query(ids, &self.patterns);
            let line_assignments = viterbi_select(ids, &matches, &self.vocab);
            assignments.extend(line_assignments);
        }

        // Write to .csm file
        self.write_csm_file(output_path, &tokenized_ids, &semantic_records)
    }

    fn build_semantic_record(&self, line_idx: usize, tokens: &[String]) -> Result<SemanticRecord, CsmError> {
        Ok(SemanticRecord {
            source_id: format!("line_{}", line_idx),
            offset: line_idx as u64,
            line_number: Some(line_idx as u64),
            ingested_at: 0,
            log_timestamp: None,
            pattern_id: 0,
            template: tokens.join(" "),
            domain: self.domain,
            slots: Vec::new(),
            raw_token_count: tokens.len() as u16,
            encoded_bits: 0,
            compress_ratio: 1.0,
            features: None,
        })
    }

    fn write_csm_file<P: AsRef<Path>>(
        &mut self, 
        output_path: P, 
        tokenized_ids: &[Vec<VocabId>], 
        semantic_records: &[SemanticRecord]
    ) -> Result<(), CsmError> {
        let mut file = File::create(output_path)?;
        
        // Prepare sections
        let vocab_section = SectionWriter::write_vocab_section(&self.vocab)?;
        let pattern_section = SectionWriter::write_pattern_section(&self.patterns)?;
        let slot_section = SectionWriter::write_slot_section(&self.patterns)?;
        
        // Calculate slot count (unique slots across all patterns)
        let slot_count = {
            use std::collections::HashSet;
            let mut unique_slots = HashSet::new();
            for pattern_id in 0..self.patterns.len() as csm_core::PatternId {
                if let Some(pattern) = self.patterns.get(pattern_id) {
                    if !pattern.deprecated {
                        for (_, slot_type) in &pattern.slot_schema.slots {
                            unique_slots.insert(slot_type.clone());
                        }
                    }
                }
            }
            unique_slots.len() as u32
        };
        
        let mut data_section = Vec::new();
        
        // Process each line
        for (_line_idx, (ids, record)) in tokenized_ids.iter().zip(semantic_records.iter()).enumerate() {
            // Get assignments for this line
            let matches = self.fst_engine.query(ids, &self.patterns);
            let line_assignments = viterbi_select(ids, &matches, &self.vocab);
            
            // Write line metadata
            let line_meta = LineMetadata {
                source_offset: record.offset,
                log_ts: record.log_timestamp.unwrap_or(0),
                token_count_orig: record.raw_token_count,
            };
            let meta_bytes = line_meta.to_bytes();
            let meta_len = meta_bytes.len() as u16;
            
            // Write LINE_META_LEN
            data_section.extend_from_slice(&meta_len.to_le_bytes());
            
            // Write LINE_META
            data_section.extend_from_slice(&meta_bytes);
            
            // Encode assignments for this line
            let mut line_bit_writer = BitWriter::new();
            for assignment in &line_assignments {
                self.encode_assignment_to_writer(&mut line_bit_writer, assignment)?;
            }
            let bit_data = line_bit_writer.finish();
            
            // Write BIT_STREAM
            data_section.extend_from_slice(&bit_data);
            
            // Calculate padding to 4-byte boundary
            let current_len = data_section.len();
            let padding_needed = (4 - (current_len % 4)) % 4;
            for _ in 0..padding_needed {
                data_section.push(0);
            }
            
            // Calculate CRC32C of LINE_META + BIT_STREAM
            let crc_start = data_section.len() - meta_bytes.len() - bit_data.len() - padding_needed;
            let crc_data = &data_section[crc_start..data_section.len() - padding_needed];
            let crc = crc32fast::hash(crc_data);
            
            // Write LINE_CRC32C
            data_section.extend_from_slice(&crc.to_le_bytes());
        }
        
        // Calculate section offsets with padding
        let mut vocab_section_padded = vocab_section;
        SectionWriter::pad_to_alignment(&mut vocab_section_padded, 8);
        
        let header_offset = 0u64;
        let vocab_offset = header_offset + CsmHeader::SIZE as u64;
        let pattern_offset = vocab_offset + vocab_section_padded.len() as u64;
        
        let mut pattern_section_padded = pattern_section;
        SectionWriter::pad_to_alignment(&mut pattern_section_padded, 64);
        
        let slot_offset = pattern_offset + pattern_section_padded.len() as u64;
        
        let mut slot_section_padded = slot_section;
        SectionWriter::pad_to_alignment(&mut slot_section_padded, 8);
        
        let data_offset = slot_offset + slot_section_padded.len() as u64;
        
        // Calculate encoded token count before padding
        let token_count_encoded = data_section.len() as u64;
        
        let mut data_section_padded = data_section;
        SectionWriter::pad_to_alignment(&mut data_section_padded, 8);
        
        // Write footer
        let footer_section = self.write_footer_section(&data_section_padded)?;
        
        // Create and write header
        let mut header = CsmHeader::default();
        header.domain = self.domain as u8;
        header.vocab_size = self.vocab.len() as u32;
        header.pattern_count = self.patterns.len() as u32;
        header.slot_count = slot_count;
        header.flags = flags::MULTI_TIER_PACK | flags::VITERBI_SELECT;
        header.token_count_original = semantic_records.iter().map(|r| r.raw_token_count as u64).sum();
        header.token_count_encoded = token_count_encoded;
        header.vocab_fingerprint = self.vocab.fingerprint;
        header.compression_ratio = if header.token_count_original > 0 {
            token_count_encoded as f32 / header.token_count_original as f32
        } else {
            1.0
        };
        header.section_offset_vocab = vocab_offset;
        header.section_offset_pattern = pattern_offset;
        header.section_offset_slot = slot_offset;
        header.section_offset_data = data_offset;
        header.section_offset_index = 0;
        header.build_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        
        // Compute header CRC32c (over bytes 0x00-0x77, store at 0x78)
        let mut header_bytes = header.to_bytes();
        let crc = crc32fast::hash(&header_bytes[0..124]);
        header_bytes[120..124].copy_from_slice(&crc.to_le_bytes());
        
        // Write all sections
        file.write_all(&header_bytes)?;
        file.write_all(&vocab_section_padded)?;
        file.write_all(&pattern_section_padded)?;
        file.write_all(&slot_section_padded)?;
        file.write_all(&data_section_padded)?;
        file.write_all(&footer_section)?;
        
        Ok(())
    }
    
    fn write_footer_section(&self, data_section: &[u8]) -> Result<Vec<u8>, CsmError> {
        let mut footer = Vec::new();
        
        // [u64 DATA_CRC32C_FULL] — CRC of entire DATA_SECTION
        let data_crc = crc32fast::hash(data_section);
        footer.extend_from_slice(&data_crc.to_le_bytes());
        
        // [u32 TOTAL_SECTIONS]
        footer.extend_from_slice(&6u32.to_le_bytes());
        
        // [u32 FOOTER_MAGIC] = 0x43534D45 ("CSME")
        footer.extend_from_slice(&0x43534D45u32.to_le_bytes());
        
        Ok(footer)
    }

    fn encode_assignment_to_writer(&self, writer: &mut BitWriter, assignment: &Assignment) -> Result<(), CsmError> {
        match &assignment.kind {
            AssignmentKind::Pattern { id } => {
                writer.write_bits(0, 2);
                writer.write_tiered(*id as u32, 0);
            }
            AssignmentKind::Token { id } => {
                writer.write_bits(1, 2);
                writer.write_tiered(*id as u32, 0);
            }
            AssignmentKind::Fallback { level } => {
                writer.write_bits(3, 2);
                writer.write_bits(*level as u32, 2);
            }
        }
        Ok(())
    }
}
