
extern crate array_tool;
extern crate tags;
extern crate walkdir;

use array_tool::vec::*;
use walkdir::{DirEntry, WalkDir};

// For testing
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::rc;
use std::time::SystemTime;

struct Index {
    pub map: HashMap<String, u64>,
    pub file_map: HashMap<String, Vec<String>>,
    pub song_list: HashMap<String, HashMap<String, Vec<String>>>,
}

impl Index {
    pub fn add_map(&mut self, tags: &[String], path: String) {
        for s in tags {
            for word in s.split(" ") {
                self.file_map
                    .entry(word.to_string())
                    .or_insert(Vec::new())
                    .push(path.clone());
            }
        }
    }

    pub fn search(&self, query: String) -> Vec<Vec<String>> {
        query.to_lowercase()
            .split(" ")
            .map(|word| self.file_map.get(word)
                .and_then(|vec| Some(vec.clone()))
                .unwrap_or(Vec::new()))
            .collect()
    }
}

trait FileHandler {
    fn write(&self, entry: &DirEntry, index: &mut Index, file: &mut File);
}

struct DefaultHandler;

impl FileHandler for DefaultHandler {
    fn write(&self, _entry: &DirEntry, _index: &mut Index, _file: &mut File) {}
}

struct MusicHandler;

impl FileHandler for MusicHandler {
    #[allow(unused_must_use)]
    fn write(&self, entry: &DirEntry, index: &mut Index, file: &mut File) {
        // println!("Reading file {}", entry.path().display());
        match tags::load(entry.path()) {
            Ok(music_file) => {
                let tag = music_file.tag();

                let artist = tag.artist().unwrap_or("Unkown".to_string());
                let album = tag.album().unwrap_or("Unknown".to_string());
                let title = tag.title().unwrap_or("Unknown".to_string());

                let path_string = entry.path()
                    .to_str()
                    .unwrap()
                    .to_string();

                index.add_map(&[title.to_lowercase(), artist.to_lowercase(), album.to_lowercase()], path_string);

                index.song_list
                    .entry(artist)
                    .or_insert(HashMap::new())
                    .entry(album)
                    .or_insert(Vec::new())
                    .push(title);
            },
            Err(ref e) if e.kind() == io::ErrorKind::Other => {
                file.write(format!("Unrecognized Music: {}\n", entry.path().display()).as_bytes());
            },
            Err(e) => {
                file.write(format!("Error reading {}: {:?}\n", entry.path().display(), e).as_bytes());
            },
        }
    }
}


// TODO: I really need to put some focus into clear up the indexing system
    // Counterpoint - I really have no clue how to implement the indexing
    // I need to add some extra information to the indices to improve ranking
        // eg. what if "Aerosmith" corresponds to a song AND the artist ??? which gets played
// TODO: Improve memory efficiency (I'm having to use a lot of clones when I don't need to)
// TODO: Improve accuracy of results
    // Lowercase all words when they go into the index (TODO: possible downsides?)
    // Handle mis-spellings and short forms of words
// TODO: Make the search engine tools into a library
    // Also need to figure out server integration
// TODO: Figure out how to properly multithread the engine (or at least some parts of it)
    // I shouldn't be running the indexer every time I run 'main'
        // I should also probably save it to a file somehow

fn main() {
    // Open the output test file
    let output_file = Path::new("_files.txt");
    let mut output = match File::create(&output_file) {
        Err(_why) => panic!("couldn't create file"),
        Ok(file) => file,
    };

    // Filter the directories I'm walking through
    let root = "C:\\";
    let fs_walker = WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| is_relevant_file(e))
        .filter_map(|e| e.ok());

    // Initialize the file handlers
    let mut default_handler: rc::Rc<FileHandler> = rc::Rc::new(DefaultHandler);
    let mut handlers = generate_handlers();

    // Time everything
    let mut num_files: u64 = 0;
    let now = SystemTime::now();

    // Start working on the indexer
    let mut index = Index{ map: HashMap::new(), song_list: HashMap::new(), file_map: HashMap::new() };

    // Crawl through the filesystem
    for entry in fs_walker {
        if !entry.file_type().is_dir() {
            let path = entry.path();

            path.extension()
                .map(|ext| ext.to_str().unwrap_or(""))
                .and_then(|ext| handlers.get_mut(ext))
                .unwrap_or(&mut default_handler)
                .write(&entry, &mut index, &mut output);
        }

        num_files += 1;
    }

    if let Ok(time) = now.elapsed() {
        println!("Visited {} files in {} seconds", num_files, time.as_secs());
    } else {
        println!("Visited {} files in ERR seconds", num_files);
    }

    let search_results = index.search("imagine Dragons".to_string());
    let mut final_results = search_results[0].clone();
    for result in &search_results[1..] {
        final_results = final_results.intersect(result.to_vec());
    }
    println!("{:#?}", final_results);
    // println!("{:#?}", &search_results[3]);
    // println!("{:#?}", index.map);
}

fn generate_handlers() -> HashMap<String, rc::Rc<FileHandler>> {
    let mut handlers: HashMap<String, rc::Rc<FileHandler>> = HashMap::new();

    let music_handler = rc::Rc::new(MusicHandler{});
    handlers.insert("mp3".to_string(), music_handler.clone());
    handlers.insert("m4a".to_string(), music_handler.clone());
    handlers.insert("mp4".to_string(), music_handler.clone());

    handlers
}

// How to improve this blacklist matching
fn is_relevant_file(entry: &DirEntry) -> bool {
    if let Some(file_name) = entry.path().file_name() {
        if file_name == "$RECYCLE.BIN"
          || file_name == "Windows.old"
          || file_name == "Windows"
          || file_name == "$GetCurrent"
          || file_name == "AppData"
        {
            return false;
        }
    }

    true
}
