use rawloader::{Tag, TiffIFD};
use std::path::Path;
use std::io::{BufReader, Read};
use std::fs::File;

pub fn get_datetime(path: &Path) -> Result<String, String> {
    let f = File::open(&path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let tiff = TiffIFD::new_file(&buffer)?;
    let raw_datetime = tiff.find_entry(Tag::DateTime).ok_or("DateTime field not found")?.get_data();
    Ok(String::from_utf8(Vec::from(raw_datetime)).map_err(|e| e.to_string())?)
}
