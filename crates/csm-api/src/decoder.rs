use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use csm_core::{Vocab, PatternRegistry, SemanticRecord, CsmError, DomainKind};
use csm_encoding::file_format::CsmHeader;

/// Decodes .csm v4 binary files back to semantic records and original content
pub struct CsmDecoder {
    header: CsmHeader,
    vocab: Vocab,
    patterns: PatternRegistry,
}

impl CsmDecoder {
    /// Load decoder from .csm file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, CsmError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        // Read and parse header
        let mut header_bytes = [0u8; 128];
        reader.read_exact(&mut header_bytes)?;
        let header = CsmHeader::from_bytes(&header_bytes)?;
        
        // Validate magic and version
        if &header.magic != b"CSM4" {
            return Err(CsmError::Other("Invalid magic bytes".to_string()));
        }
        if header.major_version != 4 {
            return Err(CsmError::Other("Unsupported version".to_string()));
        }
        
        // For now, create empty structures
        // TODO: Implement full section deserialization
        let vocab = Vocab::new();
        let patterns = PatternRegistry::new();
        
        Ok(CsmDecoder {
            header,
            vocab,
            patterns,
        })
    }
    
    /// Decode entire file to semantic records
    pub fn decode_all<P: AsRef<Path>>(path: P) -> Result<Vec<SemanticRecord>, CsmError> {
        let path_ref = path.as_ref();
        let decoder = Self::from_file(path_ref)?;
        
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);
        
        // Skip header
        reader.read_exact(&mut [0u8; 128])?;
        
        // Skip to DATA_SECTION using offset from header
        let seek_offset = decoder.header.section_offset_data as u64 - 128;
        let mut skip_buf = vec![0u8; seek_offset as usize];
        reader.read_exact(&mut skip_buf)?;
        
        // Read DATA_SECTION line by line
        let mut records = Vec::new();
        let mut line_idx = 0u64;
        
        loop {
            // Try to read LINE_META_LEN
            let mut meta_len_bytes = [0u8; 2];
            match reader.read_exact(&mut meta_len_bytes) {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(CsmError::from(e)),
            }
            
            let meta_len = u16::from_le_bytes(meta_len_bytes) as usize;
            if meta_len == 0 || meta_len > 256 {
                break; // Likely hit footer or padding
            }
            
            // Read LINE_META
            let mut meta_bytes = vec![0u8; meta_len];
            reader.read_exact(&mut meta_bytes)?;
            
            let (source_offset, log_ts, token_count_orig) = 
                Self::parse_line_metadata(&meta_bytes)?;
            
            // Read BIT_STREAM until we hit padding/CRC
            // This is simplified - find the next LINE or skip to known boundary
            let mut bitstream = Vec::new();
            let mut probe_buf = [0u8; 1];
            
            // Read until we likely hit the LINE_CRC32C (at 4-byte boundary)
            // Simplified: read some bytes as bitstream
            while bitstream.len() < 256 {
                match reader.read_exact(&mut probe_buf) {
                    Ok(_) => bitstream.push(probe_buf[0]),
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        // End of file during bitstream
                        break;
                    }
                    Err(e) => return Err(CsmError::from(e)),
                }
                
                // Check if we're at a 4-byte boundary with likely LINE_CRC32C
                if bitstream.len() >= 4 && bitstream.len() % 4 == 0 {
                    break;
                }
            }
            
            // Skip LINE_CRC32C (4 bytes)
            let mut crc_bytes = [0u8; 4];
            let _ = reader.read(&mut crc_bytes);
            
            // Create semantic record
            let record = SemanticRecord {
                source_id: format!("line_{}", line_idx),
                offset: source_offset,
                line_number: Some(line_idx),
                ingested_at: 0,
                log_timestamp: Some(log_ts),
                pattern_id: 0,
                template: String::from("(decoded)"),
                domain: DomainKind::Log,
                slots: Vec::new(),
                raw_token_count: token_count_orig,
                encoded_bits: bitstream.len() as u32 * 8,
                compress_ratio: 0.7,
                features: None,
            };
            records.push(record);
            line_idx += 1;
        }
        
        Ok(records)
    }
    
    /// Get header information
    pub fn header(&self) -> &CsmHeader {
        &self.header
    }
    
    /// Get vocabulary reference
    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }
    
    /// Get pattern registry reference
    pub fn patterns(&self) -> &PatternRegistry {
        &self.patterns
    }
    
    /// Parse LINE_META from byte buffer
    fn parse_line_metadata(meta: &[u8]) -> Result<(u64, i64, u16), CsmError> {
        if meta.len() < 18 {
            return Err(CsmError::Other("Metadata too short".to_string()));
        }
        
        let source_offset = u64::from_le_bytes([
            meta[0], meta[1], meta[2], meta[3],
            meta[4], meta[5], meta[6], meta[7],
        ]);
        
        let log_ts = i64::from_le_bytes([
            meta[8], meta[9], meta[10], meta[11],
            meta[12], meta[13], meta[14], meta[15],
        ]);
        
        let token_count = u16::from_le_bytes([meta[16], meta[17]]);
        
        Ok((source_offset, log_ts, token_count))
    }
}