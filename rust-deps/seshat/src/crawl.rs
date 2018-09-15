
use std::collections::HashMap;
use std::fs;
use std::rc;

use walkdir::{DirEntry, WalkDir};

use super::index as idx;
use super::handle;


/*
TODO: Need to add in some basic multithreading of the file handling

TODO: We currently split files between handlers based on their extension
    It may be beneficial to generalize this in the future to be based on more general properties
    However, it is also possible to have "sub-handles" to split on those properties
        NOTE: We may be able to improve the interface for this, though it is a workaround
        NOTE: We also don't *need* more than extension-splitting for the moment
 */

pub trait Crawler {
    fn is_relevant_file(&self, entry: &DirEntry) -> bool;
    fn crawl(&self, fdir: WalkDir, index: &mut idx::Index, out: &mut fs::File) -> u64;
}


pub struct WindowsCrawler {
    default_handle: rc::Rc<handle::FileHandler>,
    handles: HashMap<String, rc::Rc<handle::FileHandler>>
}

impl WindowsCrawler {
    pub fn new() -> Self {
        Self {
            default_handle: rc::Rc::new(handle::DefaultFileHandler),
            handles: HashMap::new()
        }
    }

    pub fn register_handle(&mut self, exts: &[&str], handle: rc::Rc<handle::FileHandler>) {
        for ext in exts {
            self.handles.insert(ext.to_string(), handle.clone());
        }
    }
}

impl Crawler for WindowsCrawler {
    fn crawl(&self, fdir: WalkDir, index: &mut idx::Index, out: &mut fs::File) -> u64 {
        let fdir = fdir.into_iter()
                   .filter_entry(|e| self.is_relevant_file(e))
                   .filter_map(|e| e.ok());

        let mut num_files: u64 = 0;
        for entry in fdir {
            if !entry.file_type().is_dir() {
                entry.path()
                     .extension()
                     .map(|ext| ext.to_str().unwrap_or(""))
                     .and_then(|ext| self.handles.get(ext))
                     .unwrap_or(&self.default_handle)
                     .handle(&entry, index, out);

                num_files += 1;
            }
        }

        num_files
    }

    fn is_relevant_file(&self, entry: &DirEntry) -> bool {
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
}
