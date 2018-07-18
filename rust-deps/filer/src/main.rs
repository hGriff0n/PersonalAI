
extern crate tags;
extern crate walkdir;

use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

// For testing
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::collections::HashMap;

trait FileHandler {
    fn write(&self, entry: &DirEntry, file: &mut File);
}

struct DefaultHandler;

impl FileHandler for DefaultHandler {
    fn write(&self, entry: &DirEntry, file: &mut File) {
        file.write(format!("{}\n", entry.path().display()).as_bytes());
    }
}

struct MusicHandler;

// TODO: Need parsing library for this
impl FileHandler for MusicHandler {
    fn write(&self, entry: &DirEntry, file: &mut File) {
        match tags::get(entry.path()) {
            Ok(music_file) => {
                let tags = music_file.tag();

                file.write(format!("Recognized Music: {}\n", entry.path().display()).as_bytes());
                if let Some(title) = tags.title() {
                    file.write(format!("  Title: {}\n", &title).as_bytes());
                }

                if let Some(artist) = tags.artist() {
                    file.write(format!("  Artist: {}\n", &artist).as_bytes());
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
    let output_file = Path::new("files_.txt");
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
    let default_handler: Box<FileHandler> = Box::new(DefaultHandler);
    let mut handlers: HashMap<String, Box<FileHandler>> = HashMap::new();
    handlers.insert("mp3".to_string(), Box::new(MusicHandler));
    handlers.insert("m4a".to_string(), Box::new(MusicHandler));
    handlers.insert("mp4".to_string(), Box::new(MusicHandler));

    // Time everything
    let mut num_files: u64 = 0;
    let now = SystemTime::now();

    // Crawl through the filesystem
    for entry in fs_walker {
        if !entry.file_type().is_dir() {
            let path = entry.path();

            path.extension()
                .map(|ext| ext.to_str().unwrap_or(""))
                .and_then(|ext| handlers.get(ext))
                .unwrap_or(&default_handler)
                .write(&entry, &mut output);
        }

        num_files += 1;
    }


    let music = tags::File::new(Path::new("C:\\Users\\ghoop\\Desktop\\PersonalAI\\data\\2-02 Livin' On The Edge.m4a"));
    println!("{:?}", music.tag().artist());

    if let Ok(time) = now.elapsed() {
        println!("Visited {} files in {} seconds", num_files, time.as_secs());
    } else {
        println!("Visited {} files in ERR seconds", num_files);
    }
}

// How to improve this blacklist matching
fn is_relevant_file(entry: &DirEntry) -> bool {
    if let Some(file_name) = entry.path().file_name() {
        if file_name == "$RECYCLE.BIN"
          || file_name == "Windows.old"
          || file_name == "Windows"
        {
            return false;
        }
    }

    true
}
