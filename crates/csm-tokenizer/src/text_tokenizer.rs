use csm_core::*;

#[derive(Default, Clone)]
pub struct TextTokenizerConfig;

#[derive(Clone)]
pub struct TextTokenizer {
    config: TextTokenizerConfig,
}

impl DomainTokenizer for TextTokenizer {
    type Config = TextTokenizerConfig;

    fn new(config: Self::Config) -> Self {
        TextTokenizer { config }
    }

    fn tokenize<'a>(&self, _input: &'a str) -> Vec<Token<'a>> {
        // TODO: implement proper text tokenization (whitespace-based)
        vec![]
    }

    fn domain(&self) -> DomainKind {
        DomainKind::Text
    }
}