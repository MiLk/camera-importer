use std::{fs, io};
use std::path::{Path, PathBuf};
use rexif::{ExifData, ExifTag};

mod raf;

pub fn collect_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            files.append(&mut collect_files(&p)?);
        } else {
            files.push(p.canonicalize()?);
        }
    }
    Ok(files)
}

fn find_datetime(exif: &ExifData) -> Result<String, String> {
    exif.entries
        .iter()
        .find(|e| e.tag == ExifTag::DateTime)
        .map(|e| e.value.to_string())
        .ok_or_else(|| "DateTime tag not found".to_string())
}

pub fn get_datetime(p: &Path) -> Result<String, String> {
    let ext = p.extension()
        .ok_or_else(|| format!("Unable to retrieve the extension of {}", p.display()))?
        .to_string_lossy()
        .to_uppercase();
    let exif = match ext.as_str() {
        "RAF" => raf::parse_embedded_jpeg(p)?,
        "JPG" => rexif::parse_file(p).map_err(|e| e.to_string())?,
        _ => return Err(format!("Unsupported extension {} for file {}", ext, p.display()))
    };
    find_datetime(&exif)
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
