//! Public CLI boundary. Harness enforcement is deliberately not implemented yet.

/// Text and exit status to be delivered by the binary without reinterpretation.
#[derive(Debug, PartialEq, Eq)]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}

fn success(message: &str) -> CliOutput {
    CliOutput {
        exit_code: 0,
        stdout: message.to_owned(),
        stderr: String::new(),
    }
}

fn failure(exit_code: u8, message: &str) -> CliOutput {
    CliOutput {
        exit_code,
        stdout: String::new(),
        stderr: message.to_owned(),
    }
}

/// Dispatch arguments excluding the executable name. No command executes judges.
pub fn dispatch(args: &[&str]) -> CliOutput {
    match args {
        [] | ["help"] | ["--help"] | ["-h"] => success(
            "chrono-harness — spec-first foundation\n\
             Usage: chrono-harness <command>\n\
             Commands: help | --version | spec status\n\
             check is NOT IMPLEMENTED and always exits 3. See SPEC.md.\n",
        ),
        ["--version"] | ["-V"] => {
            success(concat!("chrono-harness ", env!("CARGO_PKG_VERSION"), "\n"))
        }
        ["spec", "status"] => success(
            "SPEC_STATUS=draft\n\
             ENFORCEMENT=not-implemented\n\
             HOST_REGISTRIES=proposed\n\
             CONTRACT=SPEC.md\n",
        ),
        ["check", ..] => failure(
            3,
            "E_NOT_IMPLEMENTED: check cannot judge DELTA; no validation was performed. See SPEC.md.\n",
        ),
        _ => failure(
            2,
            "E_USAGE: unsupported command or arguments; use --help.\n",
        ),
    }
}
