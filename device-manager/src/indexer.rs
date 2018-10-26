
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;
use std::path;
use std::time;
use std::thread;

use chrono;
use clap;
use futures::{future, Future, Stream};
use tokio;
use walkdir::{DirEntry, WalkDir};

use seshat::crawl::*;
use seshat::handle;
use seshat::index::IndexWriter;
use tags;

// use device::DeviceManager;
use alt_device::DeviceManager;

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

// Delay the initial loading of the index from a file for a little bit
// This helps us spawn up the server slightly faster, avoiding reconnection issues with the modalities
type LazyLoader = Box<dyn Future<Item=(time::Instant, IndexWriter), Error=()> + Send>;
pub fn load_index<'a>(args: &'a clap::ArgMatches, mut writer: IndexWriter) -> LazyLoader {
    let hour = chrono::Duration::hours(1).to_std().unwrap();
    let index_cache = args.value_of("index-cache")
        .and_then(|dst| Some(dst.to_string()));

    match index_cache {
        Some(file) =>
            Box::new(future::lazy(move || {
                let file = path::Path::new(&file);
                info!("Loading index from file `{:?}`", file);

                writer.load_file(file);
                Ok((time::Instant::now() + hour, writer))
            })),
        None => Box::new(future::lazy(|| Ok((time::Instant::now(), writer))))
    }
}

pub fn launch<'a>(device: DeviceManager, args: &'a clap::ArgMatches, writer: IndexWriter) -> impl Future {
    let hour = chrono::Duration::hours(1).to_std().unwrap();
    let week = chrono::Duration::weeks(1).to_std().expect("Unable to convert 1 week to seconds");

    // TODO: Add in ability to configure roots from commandline/config
    let crawler = create_crawler(args);
    let root_folders = extract_roots(args);

    // Load the index, then setup a periodic check every hour for reindexing
    let indexer = load_index(args, writer)
        .and_then(move |(delay, mut writer)| {
            let reindexer = tokio::timer::Interval::new(delay, hour)
                .for_each(move |_| {
                    let folders = writer.queued_folders();

                    // TODO: Perform some degree of subsumption, etc. on the roots
                    // NOTE: If I'm push on any root file, we need to erase the index

                    // NOTE: This will be removed eventually
                    let output_file = path::Path::new("_files.txt");
                    let mut output = File::create(&output_file).unwrap();

                    // Spawn-and-forget the crawling in a separate thread
                    // NOTE: Performing crawling within the sequential code-block causes tokio's processing
                    // To grind to a halt, harming system-wide uptime and responsiveness
                    // TODO: We can't do this just yet because of the borrow checker
                    // thread::spawn(move || {
                        for root in folders {
                            info!("Starting crawling of {:?}", root);
                            crawler.crawl(WalkDir::new(root), &mut writer, &mut output);
                        }

                        writer.commit();
                        debug!("Finished reindexing");
                    // });

                    Ok(())
                })
                .map_err(|_| ());
            tokio::spawn(reindexer)
        });

    // Periodically push on all root folders to force re-indexing
    // NOTE: This capability means that to support 'file-watchers', we just add an event to push the new folder on the channel
    let root_queue_instant = time::Instant::now() + week;
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
    use clap::Arg;

    app.arg(Arg::with_name("index-cache")
        .long("index-cache")
        .help("location of the index cache storage file")
        .value_name("JSON")
        .takes_value(true))
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
