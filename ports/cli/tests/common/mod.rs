#![allow(dead_code)]

use assert_cmd::Command;
use blul_core::domain::dtos::{
    blutils_output::BlutilsOutput, file_or_stdin::FileOrStdin,
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Read-only fixtures shared with `blutils-core`.
pub fn core_fixture(relative: &str) -> PathBuf {
    manifest_dir()
        .join("../../core/tests/fixtures")
        .join(relative)
}

/// Fixtures owned by the CLI tests.
pub fn cli_fixture(relative: &str) -> PathBuf {
    manifest_dir().join("tests/fixtures").join(relative)
}

pub fn blu(cwd: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_blu"));
    cmd.current_dir(cwd);
    cmd
}

/// Parse a serialized `BlutilsOutput` (JSON) through the real contract type.
pub fn parse_blutils_output(dir: &Path, json: &[u8]) -> BlutilsOutput {
    let path = dir.join("captured-stdout.json");
    std::fs::write(&path, json).unwrap();

    FileOrStdin::from_str(path.to_str().unwrap())
        .unwrap()
        .json_content::<BlutilsOutput>()
        .expect("stdout must be a valid BlutilsOutput JSON document")
}

/// Directory with fake `which`/`blastn`/`blastdbcmd` for a given scenario
/// (`tests/bin/<scenario>`). The exec bit is (re)applied here because it may
/// be lost on checkout; scripts are never rewritten at runtime to avoid
/// `ETXTBSY` races with concurrently spawned children.
#[cfg(unix)]
pub fn fake_bin(scenario: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let dir = manifest_dir().join("tests/bin").join(scenario);

    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();

        if permissions.mode() & 0o111 != 0o111 {
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).unwrap();
        }
    }

    dir
}

/// `PATH` for the child: the fake directory first, then the system dirs the
/// scripts need (`cat`). The fake `which` only resolves names inside its own
/// directory, so a host-installed BLAST+ never leaks in.
#[cfg(unix)]
pub fn fake_path(scenario: &str) -> String {
    format!("{}:/usr/bin:/bin", fake_bin(scenario).display())
}

/// Creates `<dir>/db.nsq` and `<dir>/taxdb.btd` so that
/// `validate_blast_database(<dir>/db)` succeeds; returns `<dir>/db`.
pub fn fake_blast_database(dir: &Path) -> PathBuf {
    std::fs::write(dir.join("db.nsq"), b"").unwrap();
    std::fs::write(dir.join("taxdb.btd"), b"").unwrap();
    dir.join("db")
}
