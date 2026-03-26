use csm_core::*;
use regex::Regex;
use once_cell::sync::Lazy;

// Pre-compiled regexes for performance
static TIMESTAMP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T").unwrap()
});

static IP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap()
});

static HEX_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^0x[0-9a-fA-F]+$").unwrap()
});

static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
});

#[derive(Clone)]
pub struct LogTokenizer {
    config: LogTokenizerConfig,
}

#[derive(Debug, Clone, Default)]
pub struct LogTokenizerConfig {
    // TODO: add config options
}

impl DomainTokenizer for LogTokenizer {
    type Config = LogTokenizerConfig;

    fn new(config: Self::Config) -> Self {
        LogTokenizer { config }
    }

    fn tokenize<'a>(&self, input: &'a str) -> Vec<Token<'a>> {
        // Split on whitespace
        let fields: Vec<&str> = input.split_whitespace().collect();
        let mut tokens = Vec::new();
        for field in fields {
            let (slot_type, raw) = self.classify_field(field);
            let kind = if slot_type == SlotType::Generic {
                TokenKind::Literal
            } else {
                TokenKind::Slot(slot_type)
            };
            tokens.push(Token {
                id: 0, // TODO: lookup from vocab
                raw,
                kind,
            });
        }
        tokens
    }

    fn domain(&self) -> DomainKind {
        DomainKind::Log
    }
}

impl LogTokenizer {
    fn classify_field<'a>(&self, field: &'a str) -> (SlotType, &'a str) {
        // Timestamp
        if TIMESTAMP_REGEX.is_match(field) {
            return (SlotType::Timestamp, field);
        }
        // IP
        if IP_REGEX.is_match(field) {
            return (SlotType::IpAddress, field);
        }
        // Integer
        if field.chars().all(|c| c.is_ascii_digit()) {
            return (SlotType::Integer, field);
        }
        // Float
        if field.contains('.') && field.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return (SlotType::Float, field);
        }
        // Log level
        let level = field.to_uppercase();
        if matches!(level.as_str(), "INFO" | "WARN" | "ERROR" | "DEBUG" | "TRACE") {
            return (SlotType::LogLevel, field);
        }
        // Hex
        if HEX_REGEX.is_match(field) {
            return (SlotType::Hash, field);
        }
        // Path
        if field.contains('/') || field.contains('\\') {
            return (SlotType::Path, field);
        }
        // UUID
        if UUID_REGEX.is_match(field) {
            return (SlotType::Uuid, field);
        }
        (SlotType::Generic, field)
    }
}