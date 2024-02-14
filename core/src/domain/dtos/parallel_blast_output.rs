use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ParallelBlastOutput {
    pub output_file: PathBuf,
    pub headers: Option<Vec<String>>,
}
