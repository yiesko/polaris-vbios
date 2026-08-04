//! Shared helpers for integration tests: sample-ROM extraction, temp
//! ROMs, running the real binary, parsing its JSON output, and byte
//! mutation.
//!
//! The sample collection (`samples/samples.7z`, ~860 KiB) expands to
//! `target/sample_roms/ROMs/*.rom` (339 ROMs). Extraction needs a `7z`
//! (or `7za`/`7zr`) binary; when it or the archive is missing, the
//! sample-backed tests *skip* with a notice instead of failing, so
//! `cargo test` stays usable on a machine without the tooling. CI
//! installs `p7zip-full`, so sample tests always run there.

#![allow(dead_code)] // each tests/*.rs uses a subset of these helpers

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

fn archive_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples")
        .join("samples.7z")
}

fn dest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("sample_roms")
}

fn roms_dir() -> PathBuf {
    dest_dir().join("ROMs")
}

fn done_marker() -> PathBuf {
    roms_dir().join("__done__")
}

fn find_7z() -> Option<&'static str> {
    for name in ["7z", "7za", "7zr"] {
        let ok = Command::new(name)
            .arg("i")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();
        if ok {
            return Some(name);
        }
    }
    None
}

/// Extracts the sample archive once, idempotently and atomically.
/// Several test binaries may race here; a lock serializes them and an
/// atomic parent-dir rename guarantees the result is never half-written.
/// Returns the directory holding the `.rom` files, or `None`.
fn ensure_extracted() -> Option<PathBuf> {
    static LOCK: Mutex<()> = Mutex::new(());

    if done_marker().is_file() {
        return Some(roms_dir());
    }
    let _g = LOCK.lock().unwrap();
    if done_marker().is_file() {
        return Some(roms_dir());
    }
    if !archive_path().is_file() {
        eprintln!("skipped: samples/samples.7z not present in the repo");
        return None;
    }
    let seven_z = match find_7z() {
        Some(s) => s,
        None => {
            eprintln!("skipped: no 7z binary found (sample collection needs 7z+tar to expand)");
            return None;
        }
    };

    fs::create_dir_all(dest_dir()).ok()?;
    let staging = dest_dir().join(format!(".staging-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).ok()?;

    let spawned = Command::new(seven_z)
        .args(["x", "-so"])
        .arg(archive_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut z = match spawned {
        Ok(z) => z,
        Err(_) => {
            let _ = fs::remove_dir_all(&staging);
            return None;
        }
    };
    let so = z.stdout.take().expect("piped stdout");
    let tar_ok = Command::new("tar")
        .args(["-xf", "-", "-C"])
        .arg(&staging)
        .stdin(Stdio::from(so))
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = z.wait();

    if !tar_ok {
        let _ = fs::remove_dir_all(&staging);
        eprintln!(
            "skipped: failed to expand {} (7z/tar error)",
            archive_path().display()
        );
        return None;
    }

    // Move the ROMs/ tree into place atomically.
    let from = staging.join("ROMs");
    match fs::rename(&from, roms_dir()) {
        Ok(()) => {
            let _ = fs::write(done_marker(), b"done");
        }
        Err(_) => {
            // Another process won the race to the final name.
            let _ = fs::remove_dir_all(&from);
        }
    }
    let _ = fs::remove_dir_all(&staging);
    if done_marker().is_file() {
        Some(roms_dir())
    } else {
        None
    }
}

/// Directory with the extracted ROMs, or `None` (archive/7z missing).
/// Prints a "skipped: ..." notice so failures are observable.
pub fn require_samples() -> Option<PathBuf> {
    static_extract()
}

fn static_extract() -> Option<PathBuf> {
    static ONCE: OnceLock<Option<PathBuf>> = OnceLock::new();
    ONCE.get_or_init(ensure_extracted).clone()
}

/// Every sample ROM path, sorted. Empty when samples are unavailable.
pub fn all_roms() -> Vec<PathBuf> {
    let mut v = static_extract()
        .into_iter()
        .flat_map(|d| fs::read_dir(&d).ok())
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "rom").unwrap_or(false))
        .collect::<Vec<_>>();
    v.sort();
    v
}

/// Path of one sample ROM by filename; panics when the collection is
/// unavailable or the file is missing.
pub fn rom(name: &str) -> PathBuf {
    let dir = static_extract().expect("sample ROMs should be available (run with 7z installed?)");
    let p = dir.join(name);
    assert!(
        p.is_file(),
        "sample ROM '{name}' not found in {}",
        dir.display()
    );
    p
}

/// Bytes of one sample ROM.
pub fn rom_bytes(name: &str) -> Vec<u8> {
    fs::read(rom(name)).expect("read sample ROM")
}

/// Bytes of one sample ROM, or None when samples are unavailable.
pub fn try_rom_bytes(name: &str) -> Option<Vec<u8>> {
    let p = try_rom(name)?;
    fs::read(p).ok()
}

/// Path of one sample ROM by filename when samples are available.
pub fn try_rom(name: &str) -> Option<PathBuf> {
    let dir = static_extract()?;
    let p = dir.join(name);
    if p.is_file() { Some(p) } else { None }
}

/// RAII guard that removes a temp directory on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Creates a fresh per-process temp dir and returns a guard that
/// removes it when dropped. Call `.path()` to get the `&Path`.
pub fn temp_dir() -> TempDir {
    let dir = std::env::temp_dir().join(format!("polaris-tests-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    TempDir { path: dir }
}

/// Writes bytes to a fresh path under a per-process temp dir; returns it.
pub fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("polaris-tests-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir.join(name)
}

/// Runs the built binary. Returns (stdout, exit_code, stderr).
pub fn run(args: &[&str]) -> (String, i32, String) {
    let out = Command::new(bin_path())
        .args(args)
        .output()
        .expect("polaris-vbios binary runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Path of the binary under test (set by cargo for integration tests).
pub fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_polaris-vbios")
}

/// True when the process exited with the exact code.
pub fn exit_is(out: &(String, i32, String), code: i32) -> bool {
    out.1 == code
}

/// Stdout of a run.
pub fn stdout(out: &(String, i32, String)) -> &str {
    &out.0
}

/// Stderr of a run.
pub fn stderr(out: &(String, i32, String)) -> &str {
    &out.2
}

/// A flipped-byte copy (XOR so it always changes the position).
pub fn flip_byte(data: &[u8], off: usize) -> Vec<u8> {
    let mut d = data.to_vec();
    if let Some(b) = d.get_mut(off) {
        *b ^= 0xFF;
    }
    d
}

/// A truncated copy.
pub fn truncated(data: &[u8], len: usize) -> Vec<u8> {
    data[..len.min(data.len())].to_vec()
}

/// Returns the path to a sample ROM, or early-returns from the caller
/// with a "skipped" notice when the sample collection is unavailable.
///
/// # Examples
///
/// ```ignore
/// let p = try_rom!("AMD.RX590.8192.191126.rom");
/// let parsed = rom::parse_rom(&p).expect("parses");
/// ```
#[macro_export]
macro_rules! try_rom {
    ($name:expr) => {
        match $crate::common::try_rom($name) {
            Some(p) => p,
            None => {
                eprintln!("skipped: {} not available", $name);
                return;
            }
        }
    };
}
