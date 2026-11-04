use chrono_harness::retained_artifacts::{self, retention::Inventory};
use serde_json::json;
use std::{fs, io::Write, path::Path};
fn main() {
    if std::env::args().nth(1).as_deref() == Some("exit-success") {
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("emit-original-failure") {
        use std::io::Read;
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).unwrap();
        assert_eq!(input, b"exact stdin\0");
        std::io::stdout()
            .write_all(b"original output\0\xff")
            .unwrap();
        std::io::stderr().write_all(b"original failure\n").unwrap();
        std::process::exit(7);
    }
    if std::env::args().nth(1).as_deref() == Some("dispatch") {
        let args: Vec<_> = std::env::args().skip(2).collect();
        let result = chrono_harness::dispatch(&args.iter().map(String::as_str).collect::<Vec<_>>());
        print!("{}", result.stdout);
        eprint!("{}", result.stderr);
        std::process::exit(result.exit_code.into());
    }
    let root = std::env::args().nth(1).unwrap();
    let root = Path::new(&root);
    if std::env::args().nth(2).as_deref() == Some("scope-input") {
        use std::os::unix::fs::MetadataExt;
        let expected = std::env::args().nth(3).unwrap();
        let unrelated = std::env::args().nth(4).unwrap();
        let inherited = std::env::var(chrono_harness::process_fds::ENV).unwrap();
        let identities: Vec<_> = inherited
            .split(',')
            .map(|fd| {
                let fd: i32 = fd.parse().unwrap();
                use std::os::fd::FromRawFd;
                let file = std::mem::ManuallyDrop::new(unsafe { fs::File::from_raw_fd(fd) });
                let metadata = file.metadata().unwrap();
                format!("{}:{}", metadata.dev(), metadata.ino())
            })
            .collect();
        assert!(identities.contains(&expected), "{identities:?}");
        assert!(!identities.contains(&unrelated), "{identities:?}");
        let round = Inventory::adopted(root)
            .unwrap()
            .unwrap()
            .maintain()
            .unwrap();
        assert_eq!(round["remaining_outputs"], 1, "{round}");
        println!("exact input capability inherited");
        return;
    }
    if std::env::args().nth(2).as_deref() == Some("lifecycle-recovery") {
        fn inventory(root: &Path) -> Inventory {
            Inventory::adopted(root).unwrap().unwrap()
        }
        let path = ".chrono-harness/state/fixtures/result.json";
        fs::write(root.join(path), b"partial original").unwrap();
        let publication = retained_artifacts::retention::begin_adopted(
            root,
            "lifecycle-original",
            &root.join(path),
            json!({"operation":"start","outcome":"not-established"}),
        )
        .unwrap()
        .unwrap();
        drop(publication);
        inventory(root).maintain_at(u64::MAX).unwrap();
        assert_eq!(fs::read(root.join(path)).unwrap(), b"partial original");
        let reader = retained_artifacts::retention::read_guard(root, path)
            .unwrap()
            .unwrap();
        inventory(root)
            .reference_paths(&format!("recovery:{path}"), &[])
            .unwrap();
        assert_eq!(
            inventory(root).maintain_at(u64::MAX).unwrap()["remaining_outputs"],
            1
        );
        drop(reader);
        let final_round = inventory(root).maintain_at(u64::MAX).unwrap();
        assert_eq!(final_round["remaining_outputs"], 0, "{final_round}");
        assert_eq!(
            final_round["disposed"][0]["original_outcome"]["completion"],
            "not-established"
        );
        return;
    }
    if std::env::args().nth(2).as_deref() == Some("recheck") {
        // This real consumer verifies the expected failed original before it
        // publishes its own release state. Refusal/partial checks publish none.
        let declaration_path = std::env::args().nth(3).unwrap();
        let expected_path = std::env::args().nth(4).unwrap();
        let declaration_raw = fs::read(root.join(&declaration_path)).unwrap();
        let declaration: serde_json::Value = chrono_harness::json(&declaration_raw).unwrap();
        let expected: serde_json::Value =
            chrono_harness::json(&fs::read(root.join(expected_path)).unwrap()).unwrap();
        let originals = declaration["originals"].as_array().unwrap();
        assert_eq!(originals.len(), 1);
        assert_eq!(originals[0]["path"], expected["path"]);
        let path = expected["path"].as_str().unwrap();
        let original = chrono_harness::prepared::Original {
            path: path.into(),
            sha256: expected["sha256"].as_str().unwrap().into(),
        };
        let raw = chrono_harness::prepared::read_original(root, &original, None).unwrap();
        let report: serde_json::Value = chrono_harness::json(&raw).unwrap();
        assert_eq!(report["exit_code"], expected["exit_code"]);
        assert_eq!(report["response"]["status"], "failed");
        let receipt = json!({"retention_release":{"consumer":declaration["consumer"],"declaration_sha256":chrono_harness::sha256(&declaration_raw),"state":"released"},"observed_original_sha256":chrono_harness::sha256(&raw),"original_exit_code":report["exit_code"]});
        let receipt_raw = serde_json::to_vec(&receipt).unwrap();
        fs::write(root.join("actual-recheck-result.json"), &receipt_raw).unwrap();
        let request = json!({"operation":"release-consumer","declaration":{"path":declaration_path,"sha256":chrono_harness::sha256(&declaration_raw)},"receipt":{"path":"actual-recheck-result.json","sha256":chrono_harness::sha256(&receipt_raw)},"pointer":"/retention_release"});
        fs::write(
            root.join("actual-recheck-request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        let result = chrono_harness::dispatch(&[
            "retention",
            "--host-root",
            root.to_str().unwrap(),
            "--request",
            "actual-recheck-request.json",
        ]);
        print!("{}", result.stdout);
        eprint!("{}", result.stderr);
        std::process::exit(result.exit_code.into());
    }
    if std::env::args().nth(2).as_deref() == Some("fixture-consumer") {
        let path = root.join(".chrono-harness/state/fixtures/interrupted-capture");
        let publication =
            chrono_harness::retained_artifacts::retention::create_adopted_for_consumer(
                root,
                "fixture",
                &path,
                true,
                json!({"exit":null}),
                None,
                "fixture-recheck",
            )
            .unwrap()
            .unwrap();
        publication
            .write_file(&path.join("stderr"), b"original partial failure")
            .unwrap();
        println!("{}", publication.id());
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(45));
        drop(publication);
        return;
    }
    if std::env::args().nth(2).as_deref() == Some("consumer") {
        let id = std::env::args().nth(3).unwrap();
        let reader = Inventory::adopted(root)
            .unwrap()
            .unwrap()
            .consume(&id)
            .unwrap();
        println!("{id}");
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(45));
        drop(reader);
        return;
    }
    if std::env::args().nth(2).as_deref() == Some("creation-intent") {
        let publication = Inventory::adopted(root)
            .unwrap()
            .unwrap()
            .plan(
                "fixture",
                ".chrono-harness/state/fixtures/killed-before-create",
                true,
                json!({"operation":"creation","exit":null}),
            )
            .unwrap();
        println!("{}", publication.id());
        std::io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(45));
        drop(publication);
        return;
    }
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
