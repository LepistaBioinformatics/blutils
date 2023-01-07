use crate::domain::{
    dtos::blast_builder::BlastBuilder,
    entities::execute_step::{ExecuteStep, ExecutionResponse},
};
use clean_base::utils::errors::{execution_err, MappedErrors};
use subprocess::{Exec, Redirection};

pub struct ExecuteStepProcRepository {}

impl ExecuteStep for ExecuteStepProcRepository {
    fn run(
        &self,
        query_sequences: String,
        blast_config: BlastBuilder,
    ) -> Result<ExecutionResponse, MappedErrors> {
        let blast_response = match Exec::cmd("blastn")
            .stdin(&*query_sequences)
            .arg("-subject")
            .arg(&blast_config.subject_reads)
            .arg("-outfmt")
            .arg(&blast_config.out_format)
            .arg("-max_target_seqs")
            .arg("10")
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Pipe)
            .capture()
        {
            Err(err) => {
                return Err(execution_err(
                    format!(
                        "Unexpected error detected on execute blast: {err}"
                    ),
                    None,
                    None,
                ))
            }
            Ok(res) => res,
        };

        if !blast_response.success() {
            return Ok(ExecutionResponse::Fail(blast_response.stderr_str()));
        }

        Ok(ExecutionResponse::Success(blast_response.stdout_str()))
    }
}
