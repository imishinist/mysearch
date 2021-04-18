use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Token {
    pub offset_from: usize,
    pub offset_to: usize,

    pub position: usize,
    pub text: String,
    pub position_length: usize,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            offset_from: 0,
            offset_to: 0,
            position: usize::MAX,
            text: String::with_capacity(200),
            position_length: 1,
        }
    }
}

pub trait Tokenizer: 'static + Send + Sync + TokenizerClone {
    fn token_stream<'a>(&self, text: &'a str) -> BoxTokenStream<'a>;
}

pub trait TokenizerClone {
    fn box_clone(&self) -> Box<dyn Tokenizer>;
}

impl<T: Tokenizer + Clone> TokenizerClone for T {
    fn box_clone(&self) -> Box<dyn Tokenizer> {
        Box::new(self.clone())
    }
}

pub trait TokenStream {
    fn advance(&mut self) -> bool;

    fn token(&self) -> &Token;

    fn token_mut(&mut self) -> &mut Token;

    fn next(&mut self) -> Option<&Token> {
        if self.advance() {
            Some(self.token())
        } else {
            None
        }
    }

    fn process(&mut self, sink: &mut dyn FnMut(&Token)) -> u32 {
        let mut num_tokens_pushed = 0u32;
        while self.advance() {
            sink(self.token());
            num_tokens_pushed += 1u32;
        }
        num_tokens_pushed
    }
}

impl<'a> TokenStream for Box<dyn TokenStream + 'a> {
    fn advance(&mut self) -> bool {
        let token_stream: &mut dyn TokenStream = self.borrow_mut();
        token_stream.advance()
    }

    fn token<'b>(&'b self) -> &'b Token {
        let token_stream: &'b (dyn TokenStream + 'a) = self.borrow();
        token_stream.token()
    }

    fn token_mut<'b>(&'b mut self) -> &'b mut Token {
        let token_stream: &'b mut (dyn TokenStream + 'a) = self.borrow_mut();
        token_stream.token_mut()
    }
}

pub struct BoxTokenStream<'a>(Box<dyn TokenStream + 'a>);

impl<'a, T> From<T> for BoxTokenStream<'a>
where
    T: TokenStream + 'a,
{
    fn from(token_stream: T) -> BoxTokenStream<'a> {
        BoxTokenStream(Box::new(token_stream))
    }
}

impl<'a> Deref for BoxTokenStream<'a> {
    type Target = dyn TokenStream + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a> DerefMut for BoxTokenStream<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn transform<'a>(&self, token_stream: BoxTokenStream<'a>) -> BoxTokenStream<'a>;
}

pub trait TokenFilterClone {
    fn box_clone(&self) -> BoxTokenFilter;
}

impl<T: TokenFilter + Clone> TokenFilterClone for T {
    fn box_clone(&self) -> BoxTokenFilter {
        BoxTokenFilter::from(self.clone())
    }
}

pub struct BoxTokenFilter(Box<dyn TokenFilter>);

impl Deref for BoxTokenFilter {
    type Target = dyn TokenFilter;

    fn deref(&self) -> &dyn TokenFilter {
        &*self.0
    }
}

impl<T: TokenFilter> From<T> for BoxTokenFilter {
    fn from(tokenizer: T) -> Self {
        BoxTokenFilter(Box::new(tokenizer))
    }
}

pub struct TextAnalyzer {
    tokenizer: Box<dyn Tokenizer>,
    token_filters: Vec<BoxTokenFilter>,
}

impl<T: Tokenizer> From<T> for TextAnalyzer {
    fn from(tokenizer: T) -> Self {
        TextAnalyzer::new(tokenizer, Vec::new())
    }
}

impl TextAnalyzer {
    pub fn new<T: Tokenizer>(tokenizer: T, token_filters: Vec<BoxTokenFilter>) -> Self {
        TextAnalyzer {
            tokenizer: Box::new(tokenizer),
            token_filters,
        }
    }

    pub fn token_stream<'a>(&self, text: &'a str) -> BoxTokenStream<'a> {
        let mut token_stream = self.tokenizer.token_stream(text);
        for token_filter in &self.token_filters {
            token_stream = token_filter.transform(token_stream);
        }
        token_stream
    }
}

impl Clone for TextAnalyzer {
    fn clone(&self) -> Self {
        TextAnalyzer {
            tokenizer: self.tokenizer.box_clone(),
            token_filters: self
                .token_filters
                .iter()
                .map(|token_filter| token_filter.box_clone())
                .collect(),
        }
    }
}
