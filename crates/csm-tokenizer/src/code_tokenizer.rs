use csm_core::*;

#[derive(Default, Clone)]
pub struct CodeTokenizerConfig;

#[derive(Clone)]
pub struct CodeTokenizer {
    config: CodeTokenizerConfig,
}

impl DomainTokenizer for CodeTokenizer {
    type Config = CodeTokenizerConfig;

    fn new(config: Self::Config) -> Self {
        CodeTokenizer { config }
    }

    fn tokenize<'a>(&self, _input: &'a str) -> Vec<Token<'a>> {
        // TODO: implement proper code tokenization
        vec![]
    }

    fn domain(&self) -> DomainKind {
        DomainKind::Code
    }
}