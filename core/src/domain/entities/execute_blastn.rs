use crate::domain::dtos::blast_builder::BlastBuilder;

use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionResponse {
    Success(String),
    Fail(String),
}

pub trait ExecuteBlastn: Sync + Send + Debug {
    fn run(
        &self,
        query_sequences: String,
        blast_config: BlastBuilder,
        threads: usize,
    ) -> Result<ExecutionResponse, MappedErrors>;
}
