use std::path::{Path, PathBuf};
use camera_importer::*;
use std::collections::HashMap;
use chrono::NaiveDateTime;
use std::fs;

/// Default directory scanned recursively for JPG/RAF files.
const DEFAULT_SOURCE: &str = "F:/Pictures/XT30/Import";
/// Default destination root; files land in `DEST/<TARGET_SUBDIR_FORMAT>`.
const DEFAULT_DEST: &str = "F:/Pictures/XT30";
/// EXIF `DateTime` tag format (`YYYY:MM:DD HH:MM:SS`).
const DATETIME_FORMAT: &str = "%Y:%m:%d %H:%M:%S";
/// Destination sub-directory layout, relative to the destination root.
const TARGET_SUBDIR_FORMAT: &str = "%Y_%m/%Y%m%d";
/// How often (in files) to print a progress line.
const PROGRESS_INTERVAL: usize = 100;

#[derive(Debug)]
struct Picture {
    jpg_path: Option<PathBuf>,
    raf_path: Option<PathBuf>,
    datetime: NaiveDateTime
}

/// Read a file's capture timestamp, returning `None` (and logging a warning)
/// when the metadata is missing or unparseable, so one bad file does not abort
/// the whole import.
fn read_capture_datetime(path: &Path) -> Option<NaiveDateTime> {
    let raw = match get_datetime(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: skipping {}: {}", path.display(), e);
            return None;
        }
    };
    let trimmed = raw.trim_matches(char::from(0)).trim();
    match NaiveDateTime::parse_from_str(trimmed, DATETIME_FORMAT) {
        Ok(dt) => Some(dt),
        Err(e) => {
            eprintln!(
                "Warning: skipping {}: unparseable datetime {:?}: {}",
                path.display(), trimmed, e
            );
            None
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("camera-importer");

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: {} [SOURCE_DIR] [DEST_ROOT]", program);
        println!();
        println!("Organizes Fujifilm JPG/RAF files into a date-partitioned tree.");
        println!();
        println!("  SOURCE_DIR   Directory scanned recursively (default: {})", DEFAULT_SOURCE);
        println!("  DEST_ROOT    Destination root; files land in DEST_ROOT/{} (default: {})",
                 TARGET_SUBDIR_FORMAT, DEFAULT_DEST);
        return;
    }

    let source = args.get(1).map(String::as_str).unwrap_or(DEFAULT_SOURCE);
    let dest = args.get(2).map(String::as_str).unwrap_or(DEFAULT_DEST);

    let files = match collect_files(Path::new(source)) {
        Ok(files) => files,
        Err(e) => panic!("Unable to collect files from {}: {:?}", source, e)
    };

    let total = files.len();
    println!("Number of files: {}", total);

    println!("Gathering files...");
    let mut pictures: HashMap<String, Picture> = HashMap::new();
    let mut i = 0;
    for f in files {
        i += 1;
        // Key the stem case-insensitively too: on a case-insensitive filesystem
        // (Windows, macOS) `DSCF1234.JPG` and `dscf1234.jpg` are the same file, so
        // they must collide here rather than both move and silently overwrite each
        // other at the destination.
        let (stem, ext) = match f.file_stem().zip(f.extension()) {
            Some((s, e)) => (s.to_string_lossy().to_uppercase(), e.to_string_lossy().to_uppercase()),
            None => continue
        };
        if ext != "JPG" && ext != "RAF" {
            continue
        }
        match pictures.get_mut(&stem) {
            Some(pic) => {
                let slot = match ext.as_str() {
                    "JPG" => &mut pic.jpg_path,
                    "RAF" => &mut pic.raf_path,
                    _ => continue,
                };
                match slot {
                    Some(existing) => eprintln!(
                        "Warning: duplicate {} stem {:?}; keeping {}, ignoring {}",
                        ext, stem, existing.display(), f.display()
                    ),
                    None => *slot = Some(f),
                }
            }
            None => {
                let datetime = match read_capture_datetime(&f) {
                    Some(dt) => dt,
                    None => continue,
                };
                pictures.insert(stem, Picture {
                    datetime,
                    jpg_path: if ext == "JPG" { Some(f.clone()) } else { None },
                    raf_path: if ext == "RAF" { Some(f) } else { None },
                });
            }
        }
        if i % PROGRESS_INTERVAL == 0 {
            println!("Progress: {}/{}", i, total);
        }
    }
    println!("Progress: {}/{}", i, total);

    println!("Moving files...");
    let total = pictures.len();
    let mut i = 0;
    let root_target_path = Path::new(dest);
    for (_stem, pic) in pictures {
        let subdir = pic.datetime.format(TARGET_SUBDIR_FORMAT).to_string();
        let target_directory = root_target_path.join(&subdir);
        if let Err(e) = fs::create_dir_all(&target_directory) {
            eprintln!("Warning: cannot create {}: {}; skipping picture", target_directory.display(), e);
            continue;
        }
        for path in [pic.jpg_path, pic.raf_path].into_iter().flatten() {
            if let Err(e) = move_file(&path, &target_directory) {
                eprintln!("Warning: skipped {}: {}", path.display(), e);
            }
        }
        i += 1;
        if i % PROGRESS_INTERVAL == 0 {
            println!("Progress: {}/{}", i, total);
        }
    }
    println!("Progress: {}/{}", i, total);
}
