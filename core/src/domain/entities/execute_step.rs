use crate::domain::dtos::blast_builder::BlastBuilder;

use clean_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionResponse {
    Success(String),
    Fail(String),
}

pub trait ExecuteStep: Sync + Send {
    fn run(
        &self,
        query_sequences: String,
        blast_config: BlastBuilder,
    ) -> Result<ExecutionResponse, MappedErrors>;
}
