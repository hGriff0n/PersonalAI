
use std;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;

use chrono;
use clap;
use futures::{future, Future, Stream};
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
    let hour = chrono::Duration::hours(1).to_std().unwrap();
    let week = chrono::Duration::weeks(1).to_std().expect("Unable to convert 1 week to seconds");

    // TODO: Add in ability to configure roots from commandline/config
    let crawler = create_crawler(args);
    let root_folders = extract_roots(args);

    // Automatically queue the root folders if the index is empty
    let mut indexer_instant = std::time::Instant::now();
    if device.get_index().len() == 0 {
        for root_folder in &root_folders {
            device.get_index().push_folder(root_folder)
                .expect("Error in initializing index queue");
        }

    // Otherwise, we should be able to wait for a little while
    } else {
        indexer_instant += hour;
    }
    trace!("Calculated delays for indexing threads");

    // Check in every hour to possibly reindex the filesystem
    // TODO: See if there's any way I can speed this up (such as running the crawler in a different thread)
    // We really just need to spawn the crawler instances with this function, their finishing can be done at any point
    // NOTE: It looks like we just need to spawn up a bunch more futures and add them to the runtime queue
    let indexer = tokio::timer::Interval::new(indexer_instant, hour)
        .for_each(move |_| {
            let folders = writer.queued_folders();

            // TODO: Perform some degree of subsumption, etc. on the roots
            // NOTE: If I'm push on any root file, we need to erase the index

            // NOTE: This will be removed eventually
            let output_file = std::path::Path::new("_files.txt");
            let mut output = std::fs::File::create(&output_file).unwrap();

            for root in folders {
                info!("Starting crawling of {:?}", root);
                crawler.crawl(WalkDir::new(root), &mut writer, &mut output);

                // NOTE: This is an attempt to move the crawling into the tokio runtime (to speed it up a little)
                // I could also just spawn and forget a thread (not sure about the cleanup)
                // This will become much easier once the 'output' param is removed
                // tokio::spawn(
                //     future::ok(root)
                //         .and_then(|root| {
                //             crawler.crawl(WalkDir::new(root), &mut writer, &mut output);
                //             debug!("Finished crawling {}", root);
                //             Ok(())
                //         })
                //         .and_then(|root| {
                //             writer.commit();
                //         }));
            }

            writer.commit();
            debug!("Finished indexing");

            Ok(())
        });

    // Periodically push on all root folders to force re-indexing
    // NOTE: This capability means that to support 'file-watchers', we just add an event to push the new folder on the channel
    let root_queue_instant = std::time::Instant::now() + week;
    let queue_roots = tokio::timer::Interval::new(root_queue_instant, week)
        .for_each(move |_| {
            for root_folder in &root_folders {
                device.get_index().push_folder(root_folder)
                    .map_err(|_| tokio::timer::Error::shutdown())?
            }

            Ok(())
        });

    indexer.select2(queue_roots)
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
