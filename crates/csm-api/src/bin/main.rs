use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use csm_core::{Vocab, PatternRegistry, DomainKind};
use csm_api::{Encoder, CsmDecoder, PatternBuilder};

#[derive(Parser)]
#[command(name = "csm")]
#[command(about = "CSM++ v4.0 - AI-Native Semantic Compression Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a file or stdin
    Encode {
        /// Input file (stdin if not provided)
        #[arg(value_name = "FILE")]
        input: Option<PathBuf>,
        
        /// Output file (stdout if not provided)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        
        /// Domain: log, code, or text
        #[arg(short, long, default_value = "log")]
        domain: String,
        
        /// Output format: binary, arrow, json, or parquet
        #[arg(short = 'f', long, default_value = "binary")]
        format: String,
    },
    
    /// Decode a .csm file
    Decode {
        /// Input .csm file
        #[arg(value_name = "FILE")]
        input: PathBuf,
        
        /// Output file (stdout if not provided)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        
        /// Output format: text, arrow, or parquet
        #[arg(short = 'f', long, default_value = "text")]
        format: String,
    },
    
    /// Build pattern database from corpus
    Build {
        /// Input corpus file
        #[arg(value_name = "FILE")]
        corpus: PathBuf,
        
        /// Output pattern database
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        
        /// Domain: log, code, or text
        #[arg(short, long, default_value = "log")]
        domain: String,
    },
    
    /// Analyze compressed data
    Analyze {
        /// Input .csm file
        #[arg(value_name = "FILE")]
        input: PathBuf,
        
        /// Show detailed statistics
        #[arg(short, long)]
        detailed: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Encode { input, output, domain, format } => {
            println!("Encoding {} to {} format (domain: {})", 
                input.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "stdin".to_string()),
                format, domain);
            
            let domain_kind = match domain.as_str() {
                "log" => DomainKind::Log,
                "code" => DomainKind::Code,
                "text" => DomainKind::Text,
                _ => {
                    eprintln!("Invalid domain: {}. Use log, code, or text", domain);
                    return Ok(());
                }
            };
            
            // TODO: load vocab and patterns from somewhere
            let vocab = Vocab::new();
            let patterns = PatternRegistry::new();
            
            let mut encoder = Encoder::new(domain_kind, vocab, patterns)?;
            
            if let Some(input_path) = input {
                let output_path = output.unwrap_or_else(|| PathBuf::from("output.csm"));
                encoder.encode_file(&input_path, &output_path)?;
            } else {
                // TODO: read from stdin
                println!("Stdin encoding not implemented yet");
            }
        },
        Commands::Decode { input, output: _, format } => {
            println!("Decoding {} to {} format", input.display(), format);
            
            // Decode the file
            match CsmDecoder::decode_all(&input) {
                Ok(records) => {
                    // Output based on format
                    match format.as_str() {
                        "text" => {
                            // Print as text summary
                            for (i, record) in records.iter().enumerate() {
                                println!("[Line {}] offset={} ts={:?} tokens={} bits={}", 
                                         i, record.offset, record.log_timestamp, 
                                         record.raw_token_count, record.encoded_bits);
                            }
                            println!("\nDecoded {} records successfully", records.len());
                        },
                        _ => {
                            println!("Format '{}' not fully supported yet (text available)", format);
                            println!("Decoded {} records", records.len());
                        }
                    }
                },
                Err(e) => eprintln!("Decoding error: {}", e),
            }
        },
        Commands::Build { corpus, output: _, domain } => {
            println!("Building pattern database from {} (domain: {})", corpus.display(), domain);
            
            // Parse domain
            let domain_kind = match domain.as_str() {
                "log" => DomainKind::Log,
                "code" => DomainKind::Code,
                "text" => DomainKind::Text,
                _ => DomainKind::Generic,
            };
            
            // Build patterns from corpus
            match PatternBuilder::new(domain_kind) {
                Ok(mut builder) => {
                    match builder.build_from_file(&corpus) {
                        Ok(patterns) => {
                            println!("Pattern database built successfully");
                            println!("Total patterns: {}", patterns.len());
                            println!("Domain: {}", domain);
                        },
                        Err(e) => eprintln!("Build error: {}", e),
                    }
                },
                Err(e) => eprintln!("Builder initialization error: {}", e),
            }
        },
        Commands::Analyze { input, detailed } => {
            println!("Analyzing {} (detailed: {})", input.display(), detailed);
            
            // Load and analyze .csm file
            match CsmDecoder::decode_all(&input) {
                Ok(records) => {
                    println!("Analysis of {}:", input.display());
                    println!("Total records: {}", records.len());
                    
                    if records.len() > 0 {
                        let total_tokens: u32 = records.iter().map(|r| r.raw_token_count as u32).sum();
                        let total_bits: u32 = records.iter().map(|r| r.encoded_bits).sum();
                        let avg_compression = total_bits as f32 / total_tokens as f32;
                        
                        println!("Total original tokens: {}", total_tokens);
                        println!("Total encoded bits: {}", total_bits);
                        println!("Average bits per token: {:.2}", avg_compression);
                        println!("Estimated compression ratio: {:.2}x", 8.0 / avg_compression);
                        
                        if detailed {
                            println!("\nDetailed per-record analysis:");
                            for (i, record) in records.iter().enumerate() {
                                println!("  Record {}: {} tokens → {} bits (ratio: {:.2})", 
                                         i, record.raw_token_count, record.encoded_bits,
                                         record.raw_token_count as f32 / record.encoded_bits as f32 * 8.0);
                            }
                        }
                    }
                },
                Err(e) => eprintln!("Analysis error: {}", e),
            }
        },
    }
    
    Ok(())
}
