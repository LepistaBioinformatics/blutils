//! Sentinel for the legacy zymo-mock result shipped in `test/mock/output/`.
//! The file was produced by blutils 7.1.3 and its `config` lacks `isConfig`
//! and `runId`, which `BlastBuilder` now defaults (BUG-R04).

use blul_core::domain::dtos::{
    blutils_output::BlutilsOutput, file_or_stdin::FileOrStdin,
};
use std::{path::Path, str::FromStr};

fn zymo_fixture() -> FileOrStdin {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/mock/output/zymo-mock/blutils.consensus.json");

    FileOrStdin::from_str(path.to_str().unwrap()).unwrap()
}

// BUG-R04: the legacy output loads with defaulted `isConfig`/`runId`.
#[test]
fn zymo_fixture_deserializes_into_blutils_output() {
    let output = zymo_fixture()
        .json_content::<BlutilsOutput>()
        .expect("legacy fixture should deserialize");

    assert_eq!(output.results.len(), 3626);
}
