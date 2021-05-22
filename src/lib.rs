use std::{fs, io};
use std::path::{Path, PathBuf};
use rexif::ExifTag;

mod tiff;

pub fn collect_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            files.append(&mut collect_files(&p)?);
        } else {
            files.push(p.canonicalize()?.into());
        }
    }
    Ok(files)
}

pub fn get_datetime(p: &Path) -> Result<String, String> {
    let ext: &str = p.extension()
        .ok_or(format!("Unable to retrieve the extension of {}", p.display()))?
        .to_str().unwrap();
    match ext {
        "RAF" => Ok(tiff::get_datetime(&p)?),
        "JPG" => {
            let exif = rexif::parse_file(&p).unwrap();
            let dt = exif.entries.iter().find(|e| e.tag == ExifTag::DateTime);
            Ok(dt.unwrap().value.to_string())
        }
        _ => Err(format!("Unsupported extension {} for file {}", ext, p.display()))
    }
}

pub fn move_file(src: &Path, dest: &Path) -> Result<(), String> {
    if src.is_dir() {
        return Err("The source must not be a directory".into());
    }

    let d: PathBuf = if dest.is_dir() {
        let filename = src.file_name().unwrap();
        dest.join(filename)
    } else {
        dest.to_path_buf()
    };

    fs::rename(src, d).map_err(|e| e.to_string())
}