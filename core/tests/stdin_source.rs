//! `Source::from_str("-")` flips the process-global `STDIN_HAS_BEEN_USED`
//! flag, so this binary must hold exactly one test.

use blul_core::domain::dtos::file_or_stdin::{Source, StdinError};
use std::str::FromStr;

#[test]
fn source_from_str_dash_twice_returns_stdin_then_repeated_use_error() {
    assert!(matches!(Source::from_str("-"), Ok(Source::Stdin)));

    let second = Source::from_str("-");
    assert!(
        matches!(second, Err(StdinError::StdInRepeatedUse)),
        "unexpected result: {second:?}"
    );
    assert_eq!(
        second.unwrap_err().to_string(),
        "stdin argument used more than once"
    );

    assert!(
        matches!(Source::from_str("a.json"), Ok(Source::Arg(path)) if path == "a.json")
    );
}
