use rexif::ExifData;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

// RAF header layout: a fixed-position big-endian u32 pair points at an embedded
// full-resolution JPEG preview, which carries a standard EXIF block.
const JPEG_OFFSET_POS: usize = 84;
const JPEG_LENGTH_POS: usize = 88;
const HEADER_MIN_LEN: usize = JPEG_LENGTH_POS + 4;
const MAGIC: &[u8] = b"FUJIFILMCCD-RAW";

fn be_u32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Parse the EXIF block from the JPEG preview embedded in a Fujifilm RAF file.
///
/// Only the fixed-size header and the embedded JPEG region are read; the rest of
/// the raw file (tens of megabytes of sensor data) is never loaded into memory.
pub fn parse_embedded_jpeg(path: &Path) -> Result<ExifData, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;

    let mut header = [0u8; HEADER_MIN_LEN];
    file.read_exact(&mut header)
        .map_err(|_| format!("Not a valid RAF file: {}", path.display()))?;
    if &header[0..MAGIC.len()] != MAGIC {
        return Err(format!("Not a valid RAF file: {}", path.display()));
    }

    let off = be_u32(&header, JPEG_OFFSET_POS) as u64;
    let len = be_u32(&header, JPEG_LENGTH_POS) as usize;

    // Validate against the real file length before allocating, so a corrupt
    // length field cannot trigger a huge allocation or read past EOF.
    let file_len = file.metadata().map_err(|e| e.to_string())?.len();
    if off.checked_add(len as u64).filter(|&e| e <= file_len).is_none() {
        return Err(format!("RAF embedded JPEG out of bounds in {}", path.display()));
    }

    file.seek(SeekFrom::Start(off)).map_err(|e| e.to_string())?;
    let mut jpeg = vec![0u8; len];
    file.read_exact(&mut jpeg).map_err(|e| e.to_string())?;

    rexif::parse_buffer(&jpeg).map_err(|e| e.to_string())
}
