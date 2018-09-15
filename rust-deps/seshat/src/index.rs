
use std::collections::HashMap;
use std::fs;
use std::io;

use serde_json;


// TODO: Find a way to "shard" the database into multiple files (for memory and distributed)
// TODO: Find a way to minimize file recalculations during crawling

pub type Element = String;
pub type ElementList = Vec<Element>;

#[derive(Serialize, Deserialize)]
pub struct Index {
    pub file_map: HashMap<String, ElementList>
}

impl Index {
    pub fn new() -> Self {
        Self{
            file_map: HashMap::new()
        }
    }

    pub fn from_file(path: &str) -> Self {
        fs::File::open(path)
            .and_then(|file| serde_json::from_reader(file)
                .map_err(|err| err.into()))
            .unwrap_or(Self::new())
    }

    pub fn write_file(&self, path: &str) -> Result<(), io::Error> {
        fs::File::create(path)
            .and_then(|file| serde_json::to_writer(file, self)
                .map_err(|err| err.into()))
            .and_then(|_| Ok(()))
    }

    pub fn add(&mut self, tag: &str, path: String) -> &mut Self {
        for word in tag.to_lowercase().split(" ") {
            self.file_map
                .entry(word.to_string())
                .or_insert(Vec::new())
                .push(path.clone());
        }

        self
    }

    pub fn retrieve(&self, query: &str) -> Vec<ElementList> {
        query.to_lowercase()
            .split(" ")
            .map(|word| self.file_map.get(word)
                .and_then(|vec| Some(vec.clone()))
                .unwrap_or(Vec::new()))
            .collect()
    }
}
