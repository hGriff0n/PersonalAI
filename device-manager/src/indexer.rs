
use std::io::ErrorKind;
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

use device::DeviceManager;

// Create the fs crawler according to the configuration
fn create_crawler<'a>(_args: &'a clap::ArgMatches) -> impl Crawler {
    trace!("Creating the windows crawler for the file system");
    let mut crawler = WindowsCrawler::new();

    trace!("Registering file handles for `mp3`, `mp4`, and `m4a` file types");
    crawler.register_handle(&["mp3", "mp4", "m4a"], Arc::new(MusicHandler));

    crawler
}

// Delay the initial loading of the index from a file for a little bit
// This helps us spawn up the server slightly faster, avoiding reconnection issues with the modalities
type LazyLoader = Box<dyn Future<Item=(time::Instant, IndexWriter), Error=()> + Send>;
pub fn load_index<'a>(args: &'a clap::ArgMatches, mut writer: IndexWriter) -> LazyLoader {
    let hour = chrono::Duration::hours(1).to_std().unwrap();
    let index_cache = args.value_of("index-cache")
        .and_then(|dst| Some(dst.to_string()));

    match index_cache {
        Some(file) => {
            trace!("Found configuration for index cache file. Spawning task to load index from file `{:?}`", file);

            Box::new(future::lazy(move || {
                info!("Loading index cache file `{:?}`", file);
                let file = path::Path::new(&file);
                writer.load_file(file);
                Ok((time::Instant::now() + hour, writer))
            }))
        },
        None => Box::new(future::lazy(|| Ok((time::Instant::now(), writer))))
    }
}

pub fn launch<'a>(device: DeviceManager, args: &'a clap::ArgMatches, writer: IndexWriter) -> impl Future {
    trace!("Launching indexer task system");

    // Create indexer constants
    let hour = chrono::Duration::hours(1).to_std().unwrap();
    let week = chrono::Duration::weeks(1).to_std().expect("Unable to convert 1 week to seconds");

    // Extract the root folders from the configuration, allowing for no-values to take the default root
    let root_folders: Option<Vec<String>> = args.values_of("index-root")
        .and_then(|roots| Some(roots.map(|s| s.to_string()).collect()));
    if root_folders.is_none() {
        debug!("Configuration did not specify a value for `index-root`. Assuming system default root");
    }
    let root_folders = root_folders.unwrap_or(vec![DEFAULT_ROOT.to_string()]);
    debug!("Extracted root folders for index crawling operations: {:?}", root_folders);

    // Create the crawler
    let crawler = create_crawler(args);

    // Load the index, then setup a periodic check every hour for reindexing
    let indexer = load_index(args, writer)
        .and_then(move |(delay, mut writer)| {
            trace!("Spawning reindexer tasks on hourly timetable. Next task in {:?}", delay);
            tokio::timer::Interval::new(delay, hour)
                .for_each(move |_| {
                    let folders = writer.queued_folders();
                    trace!("Performing reindexing on the following folders: {:?}", folders);

                    // TODO: Perform some degree of subsumption, etc. on the roots
                    // NOTE: If I'm pushing on any root file, we need to erase the index

                    // Spawn-and-forget the crawling in a separate thread
                    // NOTE: Performing crawling within the sequential code-block causes tokio's processing
                    // To grind to a halt, harming system-wide uptime and responsiveness
                    // TODO: We can't do this just yet because of the borrow checker
                    // thread::spawn(move || {
                        for root in folders {
                            info!("Starting crawling of {:?}", root);
                            crawler.crawl(WalkDir::new(root), &mut writer);
                        }

                        trace!("Commiting reindexing changes to index term map");
                        writer.commit();
                    // });

                    Ok(())
                })
                .map_err(|_| ())
        });

    // Periodically push on all root folders to force re-indexing
    // NOTE: This capability means that to support 'file-watchers', we just add an event to push the new folder on the channel
    let root_queue_instant = time::Instant::now() + week;
    trace!("Spawning indexer task to automatically refresh the filesystem data every {:?} (next: {:?})", week, root_queue_instant);
    let queue_roots = tokio::timer::Interval::new(root_queue_instant, week)
        .for_each(move |_| {
            trace!("Adding root folders to reindex queue to refresh filesystem data: {:?}", root_folders);
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
        .arg(Arg::with_name("index-root")
            .long("index-root")
            .help("Root folder path for index crawling")
            .takes_value(true)
            .multiple(true)
            .number_of_values(1)
            .use_delimiter(true))
}


// Specify all the file handlers for the index system
struct MusicHandler;
impl handle::FileHandler for MusicHandler {
    #[allow(unused_must_use)]
    fn handle(&self, entry: &DirEntry, idx: &mut IndexWriter) {
        // println!("Reading file {}", entry.path().display());
        match tags::load(entry.path()) {
            Ok(music_file) => {
                let tag = music_file.tag();

                let artist = tag.artist().unwrap_or("Unkown".to_string());
                let album = tag.album().unwrap_or("Unknown".to_string());
                let title = tag.title().unwrap_or("Unknown".to_string());

                if let Some(path_string) = entry.path().to_str() {
                    let path_string = path_string.to_string();

                    trace!("Parsed music file {:?} (artist={:?}, album={:?}, title={:?})", path_string, artist, album, title);

                    idx.add(&title, path_string.clone())
                        .add(&artist, path_string.clone())
                        .add(&album, path_string.clone());
                }
            },
            Err(ref e) if e.kind() == ErrorKind::Other => {
                error!("Unrecognized music file found: {}", entry.path().display());
            },
            Err(e) => {
                error!("Error reading file {}: {:?}", entry.path().display(), e);
            },
        }
    }
}

// Specify the default system root folder (for if none is specified in config)
#[cfg(unix)]
const DEFAULT_ROOT: &'static str = "/";
#[cfg(windows)]
const DEFAULT_ROOT: &'static str = "C:\\";
