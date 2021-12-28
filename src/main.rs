use std::path::{Path, PathBuf};
use camera_importer::*;
use std::collections::HashMap;
use std::fmt::Debug;
use chrono::NaiveDateTime;
use std::fs;

#[derive(Debug)]
struct Picture {
    jpg_path: Option<PathBuf>,
    raf_path: Option<PathBuf>,
    datetime: NaiveDateTime
}

fn main() {
    let files = match collect_files(Path::new("F:/Pictures/XT30/Import")) {
        Ok(files) => files,
        Err(e) => panic!("Unable to collect files: {:?}", e)
    };

    let total = files.len();
    println!("Number of files: {}", total);

    println!("Gathering files...");
    let mut pictures: HashMap<String, Picture> = HashMap::new();
    let mut i = 0;
    for f in files {
        i += 1;
        let (stem, ext) = match f.file_stem().zip(f.extension()) {
            Some((s, e)) => (s.to_str().unwrap(), e.to_str().unwrap()),
            None => continue
        };
        if ext != "JPG" && ext != "RAF" {
            continue
        }
        match pictures.get_mut(stem) {
            Some(pic) =>
                match ext {
                    "JPG" => pic.jpg_path = Some(f),
                    "RAF" => pic.raf_path = Some(f),
                    _ => continue,
                }
            None => {
                let datetime_str = get_datetime(&f).unwrap();
                let trimmed = datetime_str.trim_matches(char::from(0)).trim();
                let native_datetime = NaiveDateTime::parse_from_str(trimmed, "%Y:%m:%d %H:%M:%S");
                pictures.insert(stem.to_string(), Picture {
                    datetime: native_datetime.unwrap(),
                    jpg_path: if ext == "JPG" { Some(f.clone()) } else { None },
                    raf_path: if ext == "RAF" { Some(f.clone()) } else { None },
                });
                ()
            }
        }
        if i % 100 == 0 {
            println!("Progress: {}/{}", i, total);
        }
    }
    println!("Progress: {}/{}", i, total);

    println!("Moving files...");
    let total = pictures.len();
    let mut i = 0;
    let root_target_path = Path::new("F:/Pictures/XT30");
    for (_stem, pic) in pictures {
        let target_directory = root_target_path.join(Path::new(&pic.datetime.format("%Y_%m/%Y%m%d").to_string()));
        fs::create_dir_all(&target_directory).expect("Creating target directory");
        if pic.jpg_path.is_some() {
            move_file(&pic.jpg_path.unwrap(),&target_directory).expect("Moving file:");
        }
        if pic.raf_path.is_some() {
            move_file(&pic.raf_path.unwrap(),&target_directory).expect("Moving file:");
        }
        i += 1;
        if i % 100 == 0 {
            println!("Progress: {}/{}", i, total);
        }
    }
    println!("Progress: {}/{}", i, total);
}

