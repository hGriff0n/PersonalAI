
use std;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;

use chrono;
use clap;
use futures::{Future, Stream};
use tokio;
use walkdir::{DirEntry, WalkDir};

use seshat::crawl::*;
use seshat::handle;
use seshat::index::IndexWriter;
use tags;

use super::device::DeviceManager;

// Create the fs crawler according to the configuration
fn create_crawler<'a>(_args: &'a clap::ArgMatches) -> impl Crawler {
    let mut crawler = WindowsCrawler::new();
    crawler.register_handle(&["mp3", "mp4", "m4a"], Arc::new(MusicHandler));
    crawler
}

// Parse the configuration arguments to extract the folders considered as "root" folders
fn extract_roots<'a>(_args: &'a clap::ArgMatches) -> Vec<String> {
    vec!["C:\\".to_string()]
}

pub fn launch<'a>(device: DeviceManager, args: &'a clap::ArgMatches, mut writer: IndexWriter) -> impl Future {
    let crawler = create_crawler(args);

    // TODO: Add in logging support for these conversions
    let hour = chrono::Duration::hours(1);
    let week = chrono::Duration::weeks(1);

    // TODO: Add in ability to configure roots from commandline/config
    // TODO: Remove the need to clone the roots vector
    let indexer_roots = extract_roots(args);
    let reindexer_roots = indexer_roots.clone();

    // TODO: Configure 'instant' based on how many entries are in the index
    let instant = std::time::Instant::now();

    // TODO: Make this pushing of the root nodes optional
    for root_folder in &reindexer_roots {
        device.index.root_channel.send(root_folder.to_string());
    }

    // Check in every hour to possibly reindex the filesystem
    // TODO: See if there's any way I can speed this up (such as running the crawler in a different thread)
    // We really just need to spawn the crawler instances with this function, their finishing can be done at any point
    // NOTE: It looks like we just need to spawn up a bunch more futures and add them to the runtime queue
    let indexer = tokio::timer::Interval::new(instant, hour.to_std().unwrap())
        .for_each(move |_| {
            let folders: Vec<String> = writer.root_channel.try_iter().collect();

            // TODO: Perform some degree of subsumption, etc. on the roots
            // NOTE: If I'm push on any root file, we need to erase the index

            let output_file = std::path::Path::new("_files.txt");
            let mut output = std::fs::File::create(&output_file).unwrap();

            // TODO: Recognize if one of the folders is a root
            for root in folders {
                // TODO: Change the crawl function to use log instead of file
                crawler.crawl(WalkDir::new(root), &mut writer, &mut output);
            }

            Ok(())
        });

    // Periodically push on all root folders to force re-indexing
    // NOTE: This capability means that to support 'file-watchers', we just add an event to push the new folder on the channel
    let reindexer = tokio::timer::Interval::new_interval(week.to_std().expect("Unable to convert weeks"))
        .for_each(move |_| {
            for root_folder in &reindexer_roots {
                device.index.root_channel.send(root_folder.to_string())
                    .map_err(|_| tokio::timer::Error::shutdown())?
            }

            Ok(())
        });

    indexer.select2(reindexer)
}

pub fn add_args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    // use clap::Arg;

    app
}


// Specify all the file handlers for the index system
struct MusicHandler;
impl handle::FileHandler for MusicHandler {
    #[allow(unused_must_use)]
    fn handle(&self, entry: &DirEntry, idx: &mut IndexWriter, file: &mut File) {
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
            Err(ref e) if e.kind() == ErrorKind::Other => {
                file.write(format!("Unrecognized Music: {}\n", entry.path().display()).as_bytes());
            },
            Err(e) => {
                file.write(format!("Error reading {}: {:?}\n", entry.path().display(), e).as_bytes());
            },
        }
    }
}
