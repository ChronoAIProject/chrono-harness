fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(chrono_harness::wire::judge_main(
        "chrono-judge-routes",
        env!("CARGO_PKG_VERSION"),
        chrono_judge_routes::judge,
    ))
}
