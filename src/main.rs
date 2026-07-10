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

fn print_help(program: &str) {
    println!("Usage: {} [OPTIONS] [SOURCE_DIR] [DEST_ROOT]", program);
    println!();
    println!("Organizes Fujifilm JPG/RAF files into a date-partitioned tree.");
    println!();
    println!("Options:");
    println!("  -n, --dry-run   Show what would be moved without touching any files");
    println!("  -h, --help      Print this help");
    println!();
    println!("Arguments:");
    println!("  SOURCE_DIR   Directory scanned recursively (default: {})", DEFAULT_SOURCE);
    println!("  DEST_ROOT    Destination root; files land in DEST_ROOT/{} (default: {})",
             TARGET_SUBDIR_FORMAT, DEFAULT_DEST);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args.first().map(String::as_str).unwrap_or("camera-importer");

    let mut positional: Vec<&str> = Vec::new();
    let mut dry_run = false;
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help(program);
                return;
            }
            "-n" | "--dry-run" => dry_run = true,
            other => positional.push(other),
        }
    }

    let source = positional.first().copied().unwrap_or(DEFAULT_SOURCE);
    let dest = positional.get(1).copied().unwrap_or(DEFAULT_DEST);

    if dry_run {
        println!("Dry run: no files will be moved.");
    }

    let files = match collect_files(Path::new(source)) {
        Ok(files) => files,
        Err(e) => panic!("Unable to collect files from {}: {:?}", source, e)
    };

    let total = files.len();
    println!("Number of files: {}", total);

    println!("Gathering files...");
    let mut pictures: HashMap<String, Picture> = HashMap::new();
    let mut skipped_no_datetime = 0usize;
    let mut duplicates = 0usize;
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
                    Some(existing) => {
                        eprintln!(
                            "Warning: duplicate {} stem {:?}; keeping {}, ignoring {}",
                            ext, stem, existing.display(), f.display()
                        );
                        duplicates += 1;
                    }
                    None => *slot = Some(f),
                }
            }
            None => {
                let datetime = match read_capture_datetime(&f) {
                    Some(dt) => dt,
                    None => {
                        skipped_no_datetime += 1;
                        continue;
                    }
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
    let mut moved = 0usize;
    let mut move_skipped = 0usize;
    let mut i = 0;
    let root_target_path = Path::new(dest);
    for (_stem, pic) in pictures {
        let subdir = pic.datetime.format(TARGET_SUBDIR_FORMAT).to_string();
        let target_directory = root_target_path.join(&subdir);
        if !dry_run {
            if let Err(e) = fs::create_dir_all(&target_directory) {
                eprintln!("Warning: cannot create {}: {}; skipping picture", target_directory.display(), e);
                continue;
            }
        }
        for path in [pic.jpg_path, pic.raf_path].into_iter().flatten() {
            if dry_run {
                let target = target_directory.join(path.file_name().unwrap());
                if target.exists() {
                    println!("[dry-run] would skip {} (destination exists)", path.display());
                    move_skipped += 1;
                } else {
                    println!("[dry-run] would move {} -> {}", path.display(), target.display());
                    moved += 1;
                }
            } else {
                match move_file(&path, &target_directory) {
                    Ok(()) => moved += 1,
                    Err(e) => {
                        eprintln!("Warning: skipped {}: {}", path.display(), e);
                        move_skipped += 1;
                    }
                }
            }
        }
        i += 1;
        if i % PROGRESS_INTERVAL == 0 {
            println!("Progress: {}/{}", i, total);
        }
    }
    println!("Progress: {}/{}", i, total);

    let verb = if dry_run { "would move" } else { "moved" };
    println!(
        "Summary: {} pictures; {} files {}; {} skipped (no timestamp); \
         {} duplicate(s) ignored; {} destination conflict(s)/error(s)",
        total, moved, verb, skipped_no_datetime, duplicates, move_skipped
    );
}
