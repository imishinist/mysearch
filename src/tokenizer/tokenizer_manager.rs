use crate::{SimpleTokenizer, TextAnalyzer};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct TokenizerManager {
    tokenizers: Arc<RwLock<HashMap<String, TextAnalyzer>>>,
}

impl TokenizerManager {
    pub fn register<T>(&self, tokenizer_name: &str, tokenizer: T)
    where
        TextAnalyzer: From<T>,
    {
        let boxed_tokenizer: TextAnalyzer = TextAnalyzer::from(tokenizer);
        self.tokenizers
            .write()
            .unwrap()
            .insert(tokenizer_name.to_string(), boxed_tokenizer);
    }

    pub fn get(&self, tokenizer_name: &str) -> Option<TextAnalyzer> {
        self.tokenizers.read().unwrap().get(tokenizer_name).cloned()
    }
}

impl Default for TokenizerManager {
    fn default() -> Self {
        let manager = TokenizerManager {
            tokenizers: Arc::new(RwLock::new(HashMap::new())),
        };
        manager.register("default", TextAnalyzer::from(SimpleTokenizer));

        manager
    }
}
