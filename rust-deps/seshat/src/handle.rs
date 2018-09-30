
use std::fs::File;

use walkdir::DirEntry;

use super::index;

// NOTE: There's a bit of a circular dependency here (the crawler doesn't hardcode the `index` type)

pub trait FileHandler: Send + Sync {
    fn handle(&self, _entry: &DirEntry, _index: &mut index::IndexWriter, _out: &mut File) {}
}

pub struct DefaultFileHandler;
impl FileHandler for DefaultFileHandler {}
