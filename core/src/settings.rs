use const_format::formatcp;

/// ? -------------------------------------------------------------------------
/// ? Define the default file system paths
/// ? -------------------------------------------------------------------------

/// This is the base project directory.
pub const TMP_DIRECTORY: &str = ".blutils/tmp";

// Here temporary blast results are stored.
pub const BLAST_QUERIES_DIRECTORY: &str =
    &formatcp!("{:?}/queries", TMP_DIRECTORY);
