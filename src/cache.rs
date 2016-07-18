use std::env;
use std::path::PathBuf;

use walkdir::{WalkDir, WalkDirIterator, DirEntry};

use error::TealdeerError::{self, CacheError};
use types::OsType;

#[derive(Debug)]
pub struct Cache {
    url: String,
    os: OsType,
}

impl Cache {
    pub fn new<S>(url: S, os: OsType) -> Cache where S: Into<String> {
        Cache {
            url: url.into(),
            os: os,
        }
    }

    /// Return the path to the page directory.
    fn get_page_dir(&self) -> Result<PathBuf, TealdeerError> {
        if let Ok(value) = env::var("TLDR_PAGE_DIR") {
            let path = PathBuf::from(value);

            if path.exists() && path.is_dir() {
                return Ok(path)
            } else {
                return Err(CacheError(
                    "Path specified by $TLDR_PAGE_DIR \
                     does not exist or is not a directory.".into()
                ));
            }
        };
        return Err(CacheError("$TLDR_PAGES_DIR isn't set.".into()));
    }

    /// Return the platform directory.
    fn get_platform_dir(&self) -> Option<&'static str> {
        match self.os {
            OsType::Linux => Some("linux"),
            OsType::OsX => Some("osx"),
            OsType::SunOs => None, // TODO: Does Rust support SunOS?
            OsType::Other => None,
        }
    }

    /// Search for a page and return the path to it.
    pub fn find_page(&self, name: &str) -> Option<PathBuf> {
        // Build page file name
        let page_filename = format!("{}.md", name);

        // Get platform dir
        let platforms_dir = match self.get_page_dir() {
            Ok(cache_dir) => cache_dir,
            _ => return None,
        };

        // Determine platform
        let platform = self.get_platform_dir();

        // Search for the page in the platform specific directory
        if let Some(pf) = platform {
            let path = platforms_dir.join(&pf).join(&page_filename);
            if path.exists() && path.is_file() {
                return Some(path);
            }
        }

        // If platform is not supported or if platform specific page does not exist,
        // look up the page in the "common" directory.
        let path = platforms_dir.join("common").join(&page_filename);

        // Return it if it exists, otherwise give up and return `None`
        if path.exists() && path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    /// Search for a page and return the path to it, whether or not the path is exists.
    pub fn find_page_to_edit(&self, name: &str) -> Option<PathBuf> {
        let page_filename = format!("{}.md", name);
        let platforms_dir = match self.get_page_dir() {
            Ok(cache_dir) => cache_dir,
            _ => return None,
        };
        let path = platforms_dir.join("common").join(&page_filename);
        Some(path)
    }

    /// Return the available pages.
    pub fn list_pages(&self) -> Result<Vec<String>, TealdeerError> {
        // Determine platforms directory and platform
        let platforms_dir = try!(self.get_page_dir());
        let platform_dir = self.get_platform_dir();

        // Closure that allows the WalkDir instance to traverse platform
        // specific and common page directories, but not others.
        let should_walk = |entry: &DirEntry| -> bool {
            let file_type = entry.file_type();
            let file_name = match entry.file_name().to_str() {
                Some(name) => name,
                None => return false,
            };
            if file_type.is_dir() {
                if file_name == "common" {
                    return true;
                }
                if let Some(platform) = platform_dir {
                    return file_name == platform;
                }
            } else if file_type.is_file() {
                return true
            }
            false
        };

        // Recursively walk through common and (if applicable) platform specific directory
        let mut pages = WalkDir::new(platforms_dir)
                                .min_depth(1) // Skip root directory
                                .into_iter()
                                .filter_entry(|e| should_walk(e)) // Filter out pages for other architectures
                                .filter_map(|e| e.ok()) // Convert results to options, filter out errors
                                .filter_map(|e| {
                                    let path = e.path();
                                    let extension = &path.extension().and_then(|s| s.to_str()).unwrap_or("");
                                    if e.file_type().is_file() && extension == &"md" {
                                        path.file_stem().and_then(|stem| stem.to_str().map(|s| s.into()))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<String>>();
        pages.sort();
        pages.dedup();
        Ok(pages)
    }
}
