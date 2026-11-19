#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// The mock may write only when both compiler destination declarations agree.
pub fn accepts_output(argument_count: usize, output_flag: bool, emit_matches: bool) -> (valid: bool)
    ensures valid == (argument_count == 12 && output_flag && emit_matches),
{
    argument_count == 12 && output_flag && emit_matches
}
}

#[cfg(not(verus_keep_ghost))]
use std::{env, fs, fs::OpenOptions, io::Write, path::Path};

#[cfg(not(verus_keep_ghost))]
fn version() {
    println!(
        "chrono-judge-routes-test-tool {}",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(not(verus_keep_ghost))]
fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") => version(),
        Some("count-version") => {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open("version-count")
                .unwrap()
                .write_all(b"v")
                .unwrap();
            version();
        }
        Some("observe") => println!(
            "{}",
            serde_json::json!([
                args.collect::<Vec<_>>(),
                env::current_dir().unwrap(),
                env::var("EMPTY").ok(),
                env::var("ABSENT").ok()
            ])
        ),
        Some("bound") => println!("bound-rust"),
        // 外部编译接口夹具：故意允许报告成功却没有生成库。
        Some("--crate-type=lib") => {
            let argv: Vec<String> = env::args().skip(1).collect();
            let output = argv.get(10).expect("missing compiler library destination");
            let emit = format!("--emit=link={output},dep-info={output}.d");
            assert!(accepts_output(
                argv.len(),
                argv.get(9).map(String::as_str) == Some("-o"),
                argv.get(7) == Some(&emit),
            ));
            if Path::new("library.fixture").exists() {
                fs::copy("library.fixture", &argv[10]).unwrap();
            }
            fs::write(format!("{}.d", argv[10]), b"original dependency bytes").unwrap();
            println!(
                "{}",
                serde_json::json!({
                    "verus": {"version":"fixture", "commit":"fixture"},
                    "verification-results": {"encountered-error":false,
                        "encountered-vir-error":false,"success":true,"verified":1,"errors":0,
                        "is-verifying-entire-crate":true}
                })
            );
        }
        mode => panic!("unknown route test helper mode: {mode:?}"),
    }
}
