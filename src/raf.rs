use rexif::ExifData;
use std::fs;
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
pub fn parse_embedded_jpeg(path: &Path) -> Result<ExifData, String> {
    let buf = fs::read(path).map_err(|e| e.to_string())?;
    if buf.len() < HEADER_MIN_LEN || &buf[0..MAGIC.len()] != MAGIC {
        return Err(format!("Not a valid RAF file: {}", path.display()));
    }

    let off = be_u32(&buf, JPEG_OFFSET_POS) as usize;
    let len = be_u32(&buf, JPEG_LENGTH_POS) as usize;
    let end = off
        .checked_add(len)
        .filter(|&e| e <= buf.len())
        .ok_or_else(|| format!("RAF embedded JPEG out of bounds in {}", path.display()))?;

    rexif::parse_buffer(&buf[off..end]).map_err(|e| e.to_string())
}
