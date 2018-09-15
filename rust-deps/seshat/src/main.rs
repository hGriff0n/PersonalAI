
extern crate array_tool;
extern crate tags;
extern crate walkdir;

use walkdir::{DirEntry, WalkDir};

// For testing
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::rc;
use std::time::SystemTime;

pub mod index;
pub mod crawl;
pub mod handle;
mod search;

use crawl::Crawler;

// NOTE: This is a per-project implementation (ie. not part of the seshat library)
struct MusicHandler;
impl handle::FileHandler for MusicHandler {
    #[allow(unused_must_use)]
    fn handle(&self, entry: &DirEntry, idx: &mut index::Index, file: &mut File) {
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

                idx.add(&title, path_string.clone())
                   .add(&artist, path_string.clone())
                   .add(&album, path_string.clone());
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


// TODO: Figure out how to properly multithread the engine (or at least some parts of it)
    // I shouldn't be running the indexer every time I run 'main'
        // I should also probably save it to a file somehow
// TODO: Add ability to save/store index to a file
    // TODO: Add ability to offload intermediate results to files
// TODO: Improve memory efficiency (I'm having to use a lot of clones when I don't need to)
// TODO: Make the search engine tools into a library
    // `Indexer` - requires some extra indirections, maybe multithreading
        // NOTE: Also need to improve the indexing system with more clarity and extra information
        // eg. what if "Aerosmith" corresponds to a song AND the artist ??? which gets played
    // `SearchEngine` - requires some more architecture work and "experience"
// TODO: Handle mis-spellings and short forms of words
// TODO: Figure out how to distribute the seshat engine in some form

fn main() {
    // Open the output test file
    // NOTE: This isn't really necessary for final implementations (may want to remove from handle interface)
    let output_file = Path::new("_files.txt");
    let mut output = match File::create(&output_file) {
        Err(_why) => panic!("couldn't create output tracking file"),
        Ok(file) => file,
    };

    // Initialize the file handlers
    let mut crawler = crawl::WindowsCrawler::new();
    crawler.register_handle(&["mp3", "mp4", "m4a"], rc::Rc::new(MusicHandler));

    // Time everything (not necessary for final implementations)
    let now = SystemTime::now();

    // Start working on the indexer
    let mut idx = index::Index::new();
    let num_files = crawler.crawl(WalkDir::new("C:\\"), &mut idx, &mut output);

    if let Ok(time) = now.elapsed() {
        println!("Visited {} files in {} seconds", num_files, time.as_secs());
    } else {
        println!("Visited {} files in ERR seconds", num_files);
    }

    let search_results = search::default_search("imagine Dragons", &idx);
    println!("{:#?}", search_results);
}
