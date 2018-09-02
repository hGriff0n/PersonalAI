
extern crate tags;
extern crate walkdir;

use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

// For testing
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::rc;

struct Index {
    pub map: HashMap<String, u64>
}

trait FileHandler {
    fn write(&self, entry: &DirEntry, index: &mut Index, file: &mut File);
}

struct DefaultHandler;

impl FileHandler for DefaultHandler {
    fn write(&self, entry: &DirEntry, _index: &mut Index, file: &mut File) {
        // file.write(format!("{}\n", entry.path().display()).as_bytes());
    }
}

struct MusicHandler;

// TODO: Need to implement utf-8 handling for mp3 files (possibly - it seems the errors are from unicode encoding at any rate)
// TODO: Some results from mp3 parsing have extra characters appended to them (Beyonce 4: Album is reported as 4T)
    // There's some errors in the mp3 parsing (or in the music files)
impl FileHandler for MusicHandler {
    fn write(&self, entry: &DirEntry, index: &mut Index, file: &mut File) {
        // println!("Reading file {}", entry.path().display());
        match tags::get(entry.path()) {
            Ok(music_file) => {
                let tags = music_file.tag();

                // file.write(format!("Recognized Music: {}\n", entry.path().display()).as_bytes());
                if let Some(title) = tags.title() {
                    file.write(format!("  Title: {}\n", &title).as_bytes());
                }

                if let Some(artist) = tags.artist() {
                    file.write(format!("  Artist: {}\n", &artist).as_bytes());

                    index.map
                        .entry(artist)
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }

                if let Some(album) = tags.album() {
                    file.write(format!("  Album: {}\n", &album).as_bytes());
                }
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
    let mut index = Index{ map: HashMap::new() };

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
