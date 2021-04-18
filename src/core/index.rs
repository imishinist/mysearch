use crate::{Directory, RAMDirectory, TokenizerManager};

pub struct Index {
    directory: Box<dyn Directory>,
    tokenizers: TokenizerManager,
}

impl Index {
    pub fn create_in_ram() -> Index {
        let ram_directory = RAMDirectory::create();
        Index {
            directory: Box::new(ram_directory),
            tokenizers: TokenizerManager::default(),
        }
    }
}

impl Clone for Index {
    fn clone(&self) -> Self {
        Index {
            directory: self.directory.box_clone(),
            tokenizers: self.tokenizers.clone(),
        }
    }
}
