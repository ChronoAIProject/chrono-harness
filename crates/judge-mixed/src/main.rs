fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::wire::judge_main(
        "chrono-judge-mixed",
        env!("CARGO_PKG_VERSION"),
        chrono_judge_mixed::judge,
    ))
}
