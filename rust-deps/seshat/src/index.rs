use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::fs;
use std::io;
use std::path::Path;

use evmap;
use serde::{Serialize, Serializer};
use serde::ser::SerializeMap;
use serde_json;

// TODO: Find a way to "shard" the database into multiple files (for memory and distributed)
// TODO: Find a way to minimize file recalculations during crawling

pub type MetaInformation = ();
pub type Element = String;
// TODO: I don't think I need this type anymore
pub type ElementList = Vec<Element>;
type _IndexWriter = evmap::WriteHandle<String, Element, MetaInformation, RandomState>;
type _IndexReader = evmap::ReadHandle<String, Element, MetaInformation, RandomState>;

pub struct IndexWriter {
    write_handle: _IndexWriter
}

impl IndexWriter {
    pub fn new(writer: _IndexWriter) -> Self {
        Self{ write_handle: writer }
    }

    pub fn add(&mut self, tag: &str, path: String) -> &mut Self {
        for word in tag.to_lowercase().split(" ") {
            self.write_handle.insert(word.to_string(), path.clone());
        }

        self
    }

    pub fn commit(&mut self) {
        self.write_handle.refresh();
    }
}


pub struct Index {
    read_handle: _IndexReader
}

impl Index {
    pub fn new() -> (Self, IndexWriter) {
        let (reader, writer) = evmap::with_meta(());
        (Self{ read_handle: reader }, IndexWriter::new(writer))
    }

    pub fn from_file(filepath: &Path) -> (Self, IndexWriter) {
        let (reader, mut writer) = evmap::with_meta(());

        let map: HashMap<String, ElementList> = fs::File::open(filepath)
            .and_then(|file| serde_json::from_reader(file)
                .map_err(|err| err.into()))
            .unwrap_or(HashMap::new());

        for (k, v) in &map {
            for item in v {
                writer.insert(k.clone(), item.clone());
            }
        }
        writer.refresh();

        (Self{ read_handle: reader }, IndexWriter::new(writer))
    }

    pub fn write_file(&self, path: &Path) -> Result<(), io::Error> {
        fs::File::create(path)
            .and_then(|file| serde_json::to_writer(file, self)
                .map_err(|err| err.into()))
            .and_then(|_| Ok(()))
    }

    pub fn retrieve(&self, query: &str) -> Vec<ElementList> {
        query.to_lowercase()
            .split(" ")
            .map(|word| self.read_handle
                .get_and(word, |slice| slice.to_vec())
                .unwrap_or(Vec::new()))
            .collect()
    }
}

impl Serialize for Index {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.read_handle.len()))?;
        self.read_handle.for_each(|k, v| { map.serialize_entry(k, v).unwrap(); });
        map.end()
    }
}
