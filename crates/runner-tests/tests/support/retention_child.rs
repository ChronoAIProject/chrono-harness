use chrono_harness::retained_artifacts::retention::Inventory;
use serde_json::json;
use std::{fs, io::Write, path::Path};
fn main() {
    if std::env::args().nth(1).as_deref() == Some("dispatch") {
        let args: Vec<_> = std::env::args().skip(2).collect();
        let result = chrono_harness::dispatch(&args.iter().map(String::as_str).collect::<Vec<_>>());
        print!("{}", result.stdout);
        eprint!("{}", result.stderr);
        std::process::exit(result.exit_code.into());
    }
    let root = std::env::args().nth(1).unwrap();
    let root = Path::new(&root);
    if let Some(previous) = std::env::args().nth(2) {
        let result = Inventory::adopted(root)
            .unwrap()
            .unwrap()
            .migrate_policy(&fs::read(previous).unwrap())
            .unwrap();
        println!("{result}");
        return;
    }
    let path = ".chrono-harness/state/fixtures/killed";
    fs::create_dir(root.join(path)).unwrap();
    let publication = Inventory::adopted(root)
        .unwrap()
        .unwrap()
        .begin("fixture", path, json!({"original_exit":null}))
        .unwrap();
    fs::write(root.join(path).join("partial"), b"original partial output").unwrap();
    println!("{}", publication.id());
    std::io::stdout().flush().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(45));
    drop(publication);
}
