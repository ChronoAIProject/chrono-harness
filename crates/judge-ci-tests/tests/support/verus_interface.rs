//! Synthetic external compiler interface for admission tests. These bytes are
//! fixtures and never establish an actual repository proof.
use std::fs;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("fixture-verus/v1");
        return;
    }
    assert_eq!(args.len(), 12);
    let output = &args[10];
    let paths = ["src.txt", "doc.txt"];
    let list = paths.join(" ");
    let mut dependency = format!("{output}.d: {list}\n\n{output}: {list}\n\n");
    for path in paths {
        dependency.push_str(&format!("{path}:\n"));
    }
    for path in paths {
        let bytes = fs::read(path).unwrap();
        dependency.push_str(&format!(
            "# checksum:sha256={} file_len:{} {path}\n",
            chrono_harness::sha256(&bytes),
            bytes.len(),
        ));
    }
    fs::write(format!("{output}.d"), dependency).unwrap();
    fs::write(output, [0u8, 255, 128, 1]).unwrap();
    println!(
        "{}",
        serde_json::json!({
            "verus":{"version":"fixture","commit":"fixture-commit"},
            "verification-results":{
                "success":true,"encountered-error":false,"encountered-vir-error":false,
                "errors":0,"verified":1,"is-verifying-entire-crate":true
            }
        })
    );
}
