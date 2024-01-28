use colored::{ColoredString, Colorize};
use subprocess::{Exec, Redirection};
use tracing::{debug, debug_span, error, info, warn};

#[tracing::instrument(name = "Checking host requirements", skip(level))]
pub fn check_host_requirements(level: Option<&str>) {
    debug_span!("check_host_requirements");

    let dependencies =
        vec![("ncbi-blast+", "blastn"), ("ncbi-blast+", "blastdbcmd")];

    let mut missing = Vec::<(&str, &str)>::new();
    let mut installed = Vec::<(&str, &str)>::new();

    for dep in dependencies {
        let check_response = match Exec::cmd("which")
            .arg("-a")
            .arg(dep.1)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Pipe)
            .capture()
        {
            Err(err) => {
                error!("Unexpected error detected on check host system: {err}");
                return;
            }
            Ok(res) => res,
        };

        if !check_response.success() {
            missing.push(dep);
            continue;
        }

        installed.push(dep);
    }

    let logging_level = level.unwrap_or("info");
    print_responses(logging_level, "AVAILABLE".green(), installed);
    print_responses(logging_level, "MISSING".yellow(), missing);
}

fn print_responses(
    level: &str,
    group: ColoredString,
    responses: Vec<(&str, &str)>,
) {
    for (name, dep) in responses {
        match level {
            "debug" => debug!("{group}:  {dep} ({name})"),
            "info" => info!("{group}:  {dep} ({name})"),
            "warn" => warn!("{group}:  {dep} ({name})"),
            "error" => error!("{group}:  {dep} ({name})"),
            _ => info!("{group}:  {dep} ({name})"),
        }
    }
}
