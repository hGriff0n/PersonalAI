
use std::fs::File;

use walkdir::DirEntry;

use super::index;

pub trait FileHandler {
    fn handle(&self, _entry: &DirEntry, _index: &mut index::Index, _out: &mut File) {}
}

pub struct DefaultFileHandler;
impl FileHandler for DefaultFileHandler {}
