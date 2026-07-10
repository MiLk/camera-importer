# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`camera-importer` is a personal Rust CLI that organizes photos exported from a
Fujifilm X-T30 camera. It scans an import directory, reads each file's capture
timestamp from EXIF/TIFF metadata, and moves JPG + RAF files into a
date-partitioned folder tree (`YYYY_MM/YYYYMMDD/`).

## Commands

```bash
cargo build              # debug build
cargo build --release    # optimized build
cargo run                # build + run the importer (uses default paths)
cargo run -- SRC DST     # run with a custom source dir and destination root
cargo run -- --help      # usage
cargo run -- --dry-run   # preview moves without touching files
cargo test               # unit tests (src/lib.rs)
```

Unit tests live in `src/lib.rs` and build their own EXIF/RAF fixtures, so no
sample camera files are needed. `cargo check` / `cargo clippy` work as usual.

## Paths

Source and destination are optional positional CLI arguments; when omitted they
fall back to compiled-in defaults (`src/main.rs`, `DEFAULT_SOURCE` /
`DEFAULT_DEST`):

- Source (scanned recursively): `F:/Pictures/XT30/Import`
- Destination root: `F:/Pictures/XT30`

Run `camera-importer <SOURCE_DIR> <DEST_ROOT>` to override without recompiling.
The datetime format, destination sub-directory layout, and progress interval are
named constants at the top of `src/main.rs`.

## Architecture

Three source files:

- `src/main.rs` — binary entry point and orchestration. Walks the collected
  files, groups JPG/RAF pairs into a `Picture` keyed by **file stem**
  (case-insensitive, so `DSCF1234.JPG` and `DSCF1234.RAF` share one `Picture`
  and land together, and a case-only difference is treated as a collision),
  parses each timestamp once via `get_datetime`, then moves files into
  `datetime.format("%Y_%m/%Y%m%d")` subfolders.
- `src/lib.rs` (`camera_importer` crate) — reusable helpers: `collect_files`
  (recursive, canonicalizing directory walk), `get_datetime` (dispatches on
  extension: JPG → `rexif::parse_file`, RAF → `raf` module, then reads the
  `DateTime` EXIF tag via the shared `find_datetime`), and `move_file`.
- `src/raf.rs` — extracts the full-resolution JPEG preview embedded in a
  Fujifilm RAF file (offset/length are big-endian u32s at fixed positions 84/88
  in the RAF header) and parses its standard EXIF block with `rexif`. This
  replaced an earlier approach that required a local `rawloader` fork.

Timestamps are parsed with the format `%Y:%m:%d %H:%M:%S`; RAF datetime strings
are trimmed of trailing NUL bytes before parsing.

Only `.JPG` and `.RAF` extensions are handled (matched case-insensitively);
other files are skipped. A file with missing/unparseable datetime is skipped
with a warning on stderr rather than aborting the run, as is a file whose stem
collides with one already collected from another folder.
