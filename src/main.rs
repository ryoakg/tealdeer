//! An implementation of [tldr](https://github.com/tldr-pages/tldr) in Rust.
//
// Copyright (c) 2015-2016 tealdeer developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be
// copied, modified, or distributed except according to those terms.

#![deny(missing_docs, missing_debug_implementations,
        unsafe_code,
        unused_import_braces, unused_qualifications)]
#![warn(trivial_casts, trivial_numeric_casts,
        missing_copy_implementations,
        unused_extern_crates, unused_results)]

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]
#![cfg_attr(feature = "dev", warn(cast_possible_truncation, cast_possible_wrap, cast_precision_loss, cast_sign_loss,
                                  mut_mut, non_ascii_literal, option_unwrap_used, result_unwrap_used,
                                  shadow_reuse, shadow_same, unicode_not_nfc,
                                  wrong_self_convention, wrong_pub_self_convention))]

#[macro_use] extern crate log;
#[cfg(feature = "logging")] extern crate env_logger;
extern crate docopt;
extern crate ansi_term;
extern crate xdg;
extern crate curl;
extern crate rustc_serialize;
extern crate time;
extern crate walkdir;

use std::io::BufReader;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process;

use docopt::Docopt;
use ansi_term::Colour;

mod types;
mod tokenizer;
mod formatter;
mod cache;
mod error;

use tokenizer::Tokenizer;
use cache::Cache;
use error::TealdeerError::{UpdateError, CacheError};
use formatter::print_lines;
use types::OsType;

const NAME: &'static str = "tealdeer";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str = "
Usage:

    tldr [options] <command>
    tldr [options]

Options:

    -h --help           Show this screen
    -v --version        Show version information
    -l --list           List all commands in the cache
    -f --render <file>  Render a specific markdown file
    -o --os <type>      Override the operating system [linux, osx, sunos]

Examples:

    $ tldr tar
    $ tldr --list

To render a local file (for testing):

    $ tldr --render /path/to/file.md
";
const ARCHIVE_URL: &'static str = "https://github.com/tldr-pages/tldr/archive/master.tar.gz";
const MAX_CACHE_AGE: i64 = 2592000; // 30 days

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_command: Option<String>,
    flag_help: bool,
    flag_version: bool,
    flag_list: bool,
    flag_render: Option<String>,
    flag_os: Option<OsType>,
}

/// Print page by path
fn print_page(path: &Path) -> Result<(), String> {
    // Open file
    let file = try!(
        File::open(path).map_err(|msg| format!("Could not open file: {}", msg))
    );
    let reader = BufReader::new(file);

    // Create tokenizer and print output
    let mut tokenizer = Tokenizer::new(reader);
    print_lines(&mut tokenizer);

    Ok(())
}

/// Check the cache for freshness
fn check_cache(args: &Args, cache: &Cache) {
    match cache.last_update() {
        Some(ago) if ago > MAX_CACHE_AGE => {
            println!("{}", Colour::Red.paint(format!(
                "Cache wasn't updated in {} days.\n\
                 You should probably run `tldr --update` soon.\n",
                MAX_CACHE_AGE / 24 / 3600
            )));
        },
        None => {
            println!("Cache not found. Please run `tldr --update`.");
            process::exit(1);
        },
        _ => {},
    }
}

#[cfg(feature = "logging")]
fn init_log() {
    env_logger::init().unwrap();
}

#[cfg(not(feature = "logging"))]
fn init_log() { }

#[cfg(target_os = "linux")]
fn get_os() -> OsType { OsType::Linux }

#[cfg(target_os = "macos")]
fn get_os() -> OsType { OsType::OsX }

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_os() -> OsType { OsType::Other }

fn main() {
    // Initialize logger
    init_log();

    // Parse arguments
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    // Show version and exit
    if args.flag_version {
        println!("{} v{}", NAME, VERSION);
        process::exit(0);
    }

    // Specify target OS
    let os: OsType = match args.flag_os {
        Some(os) => os,
        None => get_os(),
    };

    // Initialize cache
    let cache = Cache::new(ARCHIVE_URL, os);

    // Render local file and exit
    if let Some(ref file) = args.flag_render {
        let path = PathBuf::from(file);
        if let Err(msg) = print_page(&path) {
            println!("{}", msg);
            process::exit(1);
        } else {
            process::exit(0);
        };
    }

    // List cached commands and exit
    if args.flag_list {
        // Check cache for freshness
        check_cache(&args, &cache);

        // Get list of pages
        let pages = cache.list_pages().unwrap_or_else(|e| {
            match e {
                UpdateError(msg) | CacheError(msg) => println!("Could not get list of pages: {}", msg),
            }
            process::exit(1);
        });

        // Print pages
        println!("{}", pages.join(", "));
        process::exit(0);
    }

    // Show command from cache
    if let Some(ref command) = args.arg_command {
        // Check cache for freshness
        check_cache(&args, &cache);

        // Search for command in cache
        if let Some(path) = cache.find_page(&command) {
            if let Err(msg) = print_page(&path) {
                println!("{}", msg);
                process::exit(1);
            } else {
                process::exit(0);
            }
        } else {
            println!("Page {} not found in cache", &command);
            println!("Try updating with `tldr --update`, or submit a pull request to:");
            println!("https://github.com/tldr-pages/tldr");
            process::exit(1);
        }
    }

    // Some flags can be run without a command.
    println!("{}", USAGE);
    process::exit(1);
}
