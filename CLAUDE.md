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
cargo run                # build + run the importer
```

There are no tests or lints configured. `cargo check` / `cargo clippy` work as usual.

## Hardcoded paths

Import/export paths are compiled in, not passed as arguments (`src/main.rs`):

- Source (scanned recursively): `F:/Pictures/XT30/Import`
- Destination root: `F:/Pictures/XT30`

Changing where files are read from or written to means editing these literals.

## Architecture

Three source files:

- `src/main.rs` — binary entry point and orchestration. Walks the collected
  files, groups JPG/RAF pairs into a `Picture` keyed by **file stem** (so
  `DSCF1234.JPG` and `DSCF1234.RAF` share one `Picture` and land together),
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

Only `.JPG` and `.RAF` extensions (uppercase) are handled; other files are
skipped. Note the code currently `.unwrap()`s metadata parsing, so a file with
missing/unparseable datetime will panic.
