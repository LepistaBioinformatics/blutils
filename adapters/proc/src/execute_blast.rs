use blul_core::domain::{
    dtos::blast_builder::BlastBuilder,
    entities::execute_blastn::{ExecuteBlastn, ExecutionResponse},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use subprocess::{Exec, Redirection};

#[derive(Debug)]
pub struct ExecuteBlastnProcRepository {}

impl ExecuteBlastn for ExecuteBlastnProcRepository {
    fn run(
        &self,
        query_sequences: String,
        blast_config: BlastBuilder,
        threads: usize,
    ) -> Result<ExecutionResponse, MappedErrors> {
        let blast_response = match Exec::cmd("blastn")
            .stdin(&*query_sequences)
            .arg("-db")
            .arg(&blast_config.subject_reads)
            .arg("-outfmt")
            .arg(&blast_config.out_format.to_string())
            .arg("-max_target_seqs")
            .arg(&blast_config.max_target_seqs.to_string())
            .arg("-perc_identity")
            .arg(&blast_config.perc_identity.to_string())
            .arg("-qcov_hsp_perc")
            .arg(&blast_config.query_cov.to_string())
            .arg("-strand")
            .arg(&blast_config.strand.to_string())
            .arg("-evalue")
            .arg(&blast_config.e_value.to_string())
            .arg("-word_size")
            .arg(&blast_config.word_size.to_string())
            .arg("-num_threads")
            .arg(threads.to_string())
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Pipe)
            .capture()
        {
            Err(err) => {
                return execution_err(format!(
                    "Unexpected error detected on execute blast: {err}"
                ))
                .as_error()
            }
            Ok(res) => res,
        };

        if !blast_response.success() {
            return Ok(ExecutionResponse::Fail(blast_response.stderr_str()));
        }

        Ok(ExecutionResponse::Success(blast_response.stdout_str()))
    }
}
