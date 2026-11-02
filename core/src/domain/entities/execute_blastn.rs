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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_response_serde_is_externally_tagged() {
        assert_eq!(
            serde_json::to_string(&ExecutionResponse::Success(
                "out".to_string()
            ))
            .unwrap(),
            r#"{"Success":"out"}"#
        );
        assert_eq!(
            serde_json::to_string(&ExecutionResponse::Fail("err".to_string()))
                .unwrap(),
            r#"{"Fail":"err"}"#
        );
    }

    #[test]
    fn execution_response_deserialize_tagged_object_returns_variant() {
        assert!(matches!(
            serde_json::from_str::<ExecutionResponse>(r#"{"Success":"out"}"#),
            Ok(ExecutionResponse::Success(value)) if value == "out"
        ));
        assert!(matches!(
            serde_json::from_str::<ExecutionResponse>(r#"{"Fail":"err"}"#),
            Ok(ExecutionResponse::Fail(value)) if value == "err"
        ));
    }
}
