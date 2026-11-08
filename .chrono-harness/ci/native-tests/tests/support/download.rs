//! 下载器外部接口 fixture；不访问网络，原退出和响应头由生产 CLI 消费。
use std::{env, fs, process::ExitCode};

fn run() -> Result<u8, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let selected = |flag| -> Result<&str, String> {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .ok_or("missing curl flag")?;
        args.get(index + 1)
            .map(String::as_str)
            .ok_or("missing curl value".into())
    };
    fs::write("download-calls.json", serde_json::to_vec(&args).unwrap())
        .map_err(|e| e.to_string())?;
    fs::write(selected("--dump-header")?, b"fixture response headers\n")
        .map_err(|e| e.to_string())?;
    fs::copy("download-body", selected("--output")?).map_err(|e| e.to_string())?;
    let code = fs::read_to_string("download-exit")
        .unwrap_or_else(|_| "0".into())
        .parse::<u8>()
        .map_err(|e| e.to_string())?;
    if code != 0 {
        eprintln!("fixture download failed");
    }
    Ok(code)
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
