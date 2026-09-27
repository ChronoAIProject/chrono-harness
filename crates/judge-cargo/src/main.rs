use std::{
    collections::BTreeSet,
    io::{Read, Write},
    path::Path,
};
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("chrono-judge-cargo {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if args.len() == 4 && args[..3] == ["--protocol", "chrono-judge/v1", "--policy"] {
        let mut bytes = vec![];
        if std::io::stdin()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() > 64 * 1024 * 1024
        {
            return fail("E_PROTOCOL: request read/bound");
        }
        let req = match chrono_harness::decode(&bytes) {
            Ok(r) => r,
            Err(e) => return fail(&e),
        };
        let response = chrono_judge_cargo::judge(&req, &args[3]);
        match chrono_harness::wire::canonical(&response) {
            Ok(bytes) => {
                if std::io::stdout().write_all(&bytes).is_err() {
                    return fail("E_PROTOCOL: output");
                }
            }
            Err(e) => return fail(&e),
        }
        return std::process::ExitCode::from(response.status.exit_code() as u8);
    }
    if args.len() >= 9
        && args[0] == "check"
        && args[1] == "--host-root"
        && args[3] == "--config"
        && args[5] == "--policy"
    {
        let mut selected = BTreeSet::new();
        for pair in args[7..].chunks(2) {
            if pair.len() != 2 || pair[0] != "--project" || !selected.insert(pair[1].clone()) {
                return fail("E_USAGE: explicit unique --project values required");
            }
        }
        return match chrono_judge_cargo::standalone(
            Path::new(&args[2]),
            &args[4],
            &args[6],
            &selected,
        ) {
            Ok(value) => {
                println!("{value}");
                std::process::ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::ExitCode::from(1)
            }
        };
    }
    fail(
        "E_USAGE: --protocol chrono-judge/v1 --policy PATH | check --host-root ROOT --config PATH --policy PATH --project ID ...",
    )
}
fn fail(message: &str) -> std::process::ExitCode {
    eprintln!("{message}");
    std::process::ExitCode::from(2)
}
