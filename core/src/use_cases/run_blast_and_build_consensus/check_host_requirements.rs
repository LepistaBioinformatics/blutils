use colored::{ColoredString, Colorize};
use log::{error, log, Level};
use std::str::FromStr;
use subprocess::{Exec, Redirection};

pub fn check_host_requirements() {
    let dependencies = vec![("ncbi-blast+", "blastn")];

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

    print_responses("info", "AVAILABLE".green(), installed);
    print_responses("info", "MISSING".red(), missing);
}

fn print_responses(
    level: &str,
    group: ColoredString,
    responses: Vec<(&str, &str)>,
) {
    let level = Level::from_str(level).unwrap();

    for (name, dep) in responses {
        log!(level, "{group}:  {dep} ({name})");
    }
}
