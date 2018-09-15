
use std::collections::HashMap;


// TODO: Need to better name these types
pub type Element = String;
pub type Results = Vec<Element>;

pub struct Index {
    pub file_map: HashMap<String, Results>
}

impl Index {
    pub fn new() -> Self {
        Self{
            file_map: HashMap::new()
        }
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

    pub fn retrieve(&self, query: &str) -> Vec<Results> {
        query.to_lowercase()
            .split(" ")
            .map(|word| self.file_map.get(word)
                .and_then(|vec| Some(vec.clone()))
                .unwrap_or(Vec::new()))
            .collect()
    }
}
