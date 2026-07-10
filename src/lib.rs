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

    // Never clobber an existing file: `fs::rename` replaces the destination
    // silently, so a re-import (or a same-day counter rollover) could otherwise
    // overwrite a photo already filed on a previous run. `exists()` is
    // case-insensitive on Windows/macOS, matching how those filesystems collide.
    if d.exists() {
        return Err(format!("destination already exists: {}", d.display()));
    }

    fs::rename(src, d).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh, uniquely-named scratch directory under the system temp dir.
    fn temp_subdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("camera_importer_test_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build a minimal JPEG carrying an EXIF `DateTime` tag (`YYYY:MM:DD HH:MM:SS`).
    fn exif_jpeg(datetime: &str) -> Vec<u8> {
        let mut s = datetime.as_bytes().to_vec();
        s.push(0); // trailing NUL
        let count = s.len() as u32;

        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II"); // little-endian
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes()); // offset to IFD0
        tiff.extend_from_slice(&1u16.to_le_bytes()); // one entry
        tiff.extend_from_slice(&0x0132u16.to_le_bytes()); // DateTime tag
        tiff.extend_from_slice(&2u16.to_le_bytes()); // ASCII
        tiff.extend_from_slice(&count.to_le_bytes());
        tiff.extend_from_slice(&26u32.to_le_bytes()); // value offset (8+2+12+4)
        tiff.extend_from_slice(&0u32.to_le_bytes()); // no next IFD
        tiff.extend_from_slice(&s);

        let mut payload = Vec::new();
        payload.extend_from_slice(b"Exif\x00\x00");
        payload.extend_from_slice(&tiff);

        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpg.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        jpg.extend_from_slice(&payload);
        jpg.extend_from_slice(&[0xFF, 0xD9]);
        jpg
    }

    /// Wrap a JPEG in a minimal Fujifilm RAF container, padded with fake sensor
    /// data after the preview to prove the parser does not read the whole file.
    fn raf_bytes(jpeg: &[u8]) -> Vec<u8> {
        let off: u32 = 128;
        let mut buf = vec![0u8; off as usize];
        buf[0..b"FUJIFILMCCD-RAW".len()].copy_from_slice(b"FUJIFILMCCD-RAW");
        buf[84..88].copy_from_slice(&off.to_be_bytes());
        buf[88..92].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
        buf.extend_from_slice(jpeg);
        buf.extend_from_slice(&vec![0u8; 1_000_000]); // trailing "sensor data"
        buf
    }

    #[test]
    fn collect_files_recurses_into_subdirectories() {
        let dir = temp_subdir("collect");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.txt"), b"1").unwrap();
        fs::write(dir.join("sub/b.txt"), b"2").unwrap();

        let files = collect_files(&dir).unwrap();
        assert_eq!(files.len(), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_file_moves_into_directory() {
        let dir = temp_subdir("move_into");
        let src = dir.join("a.txt");
        fs::write(&src, b"hi").unwrap();
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();

        move_file(&src, &dest).unwrap();

        assert!(dest.join("a.txt").exists());
        assert!(!src.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_file_refuses_to_overwrite_existing_destination() {
        let dir = temp_subdir("move_overwrite");
        let src = dir.join("a.txt");
        fs::write(&src, b"new").unwrap();
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("a.txt"), b"old").unwrap();

        let err = move_file(&src, &dest).unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");
        // Both files are left untouched.
        assert!(src.exists());
        assert_eq!(fs::read(dest.join("a.txt")).unwrap(), b"old");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_file_refuses_directory_source() {
        let dir = temp_subdir("move_dir_src");
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();

        assert!(move_file(&sub, &dir).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_datetime_reads_raf_embedded_jpeg() {
        let dir = temp_subdir("raf_dt");
        let path = dir.join("DSCF9999.RAF");
        fs::write(&path, raf_bytes(&exif_jpeg("2024:05:15 10:30:00"))).unwrap();

        let dt = get_datetime(&path).unwrap();
        assert_eq!(dt.trim_matches('\0').trim(), "2024:05:15 10:30:00");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_datetime_dispatch_is_case_insensitive() {
        let dir = temp_subdir("raf_lower");
        let path = dir.join("dscf9999.raf"); // lowercase extension
        fs::write(&path, raf_bytes(&exif_jpeg("2024:05:15 10:30:00"))).unwrap();

        assert!(get_datetime(&path).is_ok());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_datetime_rejects_non_raf_bytes() {
        let dir = temp_subdir("raf_bad");
        let path = dir.join("DSCF0000.RAF");
        fs::write(&path, b"not a fuji raw").unwrap();

        assert!(get_datetime(&path).is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
