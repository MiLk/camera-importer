# camera-importer

A small Rust CLI that organizes photos exported from a **Fujifilm X-T30** camera
into a tidy, date-partitioned folder tree.

It scans an import directory, reads each file's capture time from its EXIF
metadata, and moves the matching `JPG` + `RAF` files into
`YYYY_MM/YYYYMMDD/` subfolders under a destination root:

```
F:/Pictures/XT30/
└── 2024_05/
    └── 20240515/
        ├── DSCF1234.JPG
        └── DSCF1234.RAF
```

A JPG and RAF that share the same name (e.g. `DSCF1234.JPG` and `DSCF1234.RAF`)
are always moved together into the same folder.

## Installation

You need a [Rust toolchain](https://rustup.rs/). Then:

```bash
cargo build --release
```

The binary lands at `target/release/camera-importer` (or
`target\release\camera-importer.exe` on Windows). Copy it wherever you like.

## Usage

```bash
camera-importer [SOURCE_DIR] [DEST_ROOT]
camera-importer --help
```

- `SOURCE_DIR` — directory scanned **recursively** for photos.
- `DEST_ROOT` — destination root; files land in `DEST_ROOT/YYYY_MM/YYYYMMDD/`.

Both arguments are optional and positional. When omitted they fall back to
compiled-in defaults:

| Argument     | Default                  |
| ------------ | ------------------------ |
| `SOURCE_DIR` | `F:/Pictures/XT30/Import` |
| `DEST_ROOT`  | `F:/Pictures/XT30`        |

Examples:

```bash
# Use the built-in default paths
camera-importer

# Custom source and destination
camera-importer "D:/Card/DCIM" "D:/Photos"
```

Order matters: the first argument is always the source, the second the
destination. Passing a single argument sets only the source; the destination
still uses its default.

## Running on Windows from a desktop shortcut

If you launch the importer by double-clicking a desktop shortcut, you can change
the source and destination by editing the shortcut — no rebuild required.

1. Right-click the shortcut → **Properties**.
2. On the **Shortcut** tab, find the **Target** field. It normally holds just
   the path to the exe:
   ```
   "F:\Tools\camera-importer.exe"
   ```
3. Append your source and destination, each in its own quotes:
   ```
   "F:\Tools\camera-importer.exe" "F:\Pictures\XT30\Import" "F:\Pictures\XT30"
   ```
4. Click **OK**.

Notes:

- Keep each path in its own quotes — required if any path contains spaces.
- Either `\` or `/` works as a path separator on Windows.
- With no arguments the shortcut keeps using the built-in defaults, so an
  existing plain shortcut works unchanged.

### Tip: keep the window open to read messages

Double-clicking runs the app in a console window that closes the moment it
finishes, so you can't read any warnings. To keep it open, point the shortcut at
a small batch file (e.g. `run-import.bat` next to the exe) instead:

```bat
@echo off
"F:\Tools\camera-importer.exe" "F:\Pictures\XT30\Import" "F:\Pictures\XT30"
pause
```

`pause` waits for a keypress before closing, and you can change the paths any
time by opening the `.bat` in Notepad.

## What it handles

- Only `.JPG` and `.RAF` files are processed (matched case-insensitively). Every
  other file is skipped.
- Capture time comes from the EXIF `DateTime` tag. For RAF files it is read from
  the full-resolution JPEG preview embedded in the raw file.
- The run does not abort on a single bad file. A file whose timestamp is missing
  or unparseable is skipped with a warning on stderr, and the rest continue.
- If two files resolve to the same name (same stem and extension, case-insensitive)
  — for example the camera's counter rolled over across two import folders — the
  first one seen is kept and the duplicate is reported and left in place, rather
  than one silently overwriting the other.

Because it *moves* (not copies) files, they are removed from the source
directory as they are imported. Moves happen on the same drive, so source and
destination should live on the same volume.

## Development

```bash
cargo build            # debug build
cargo run              # build + run with default paths
cargo run -- SRC DST   # build + run with custom paths
cargo clippy           # lints
```

The datetime format, destination sub-directory layout, and progress interval are
named constants at the top of `src/main.rs`. See `CLAUDE.md` for a short tour of
the source layout.
