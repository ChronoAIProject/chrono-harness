use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn raw_checkout_scoped_rejects_hidden_mode_before_and_after_operations() {
    for after in [false, true] {
        let h = Host::new();
        h.git(&["config", "core.filemode", "false"]);
        let base = h.head();
        if after {
            h.write("check.sh", "chmod +x src.txt\n");
        }
        h.write("src.txt", "candidate\n");
        let candidate = h.commit();
        if !after {
            fs::set_permissions(h.root().join("src.txt"), fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        assert!(h.git(&["diff", "--name-only", &candidate, "--"]).is_empty());
        fail(&h.check(&base, &candidate), "dirty tracked");
        assert_eq!(
            fs::metadata(h.root().join("src.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o100,
            0o100
        );
    }
}

#[test]
fn raw_checkout_scoped_rejects_clean_filter_without_running_it() {
    let h = Host::new();
    h.write(".gitattributes", "src.txt filter=hide\n");
    let mut map = h.read(".chrono-harness/FILEMAP.json");
    map["files"].as_array_mut().unwrap().push(json!({"path":".gitattributes","owner":"host","surface":"product","cost":"unmeasured","edges":[]}));
    h.json(".chrono-harness/FILEMAP.json", &map);
    h.git(&["config", "filter.hide.clean", "printf 'one\\n'"]);
    let base = h.commit();
    h.write("doc.txt", "changed\n");
    let candidate = h.commit();
    h.write("src.txt", "hidden real bytes\n");
    assert!(h.git(&["diff", "--name-only", &candidate, "--"]).is_empty());
    h.git(&[
        "config",
        "filter.hide.clean",
        "echo filter-ran > .chrono-harness/state/filter-called; exit 93",
    ]);
    h.git(&["config", "filter.hide.required", "true"]);
    fail(&h.check(&base, &candidate), "dirty tracked");
    assert!(
        !h.root()
            .join(".chrono-harness/state/filter-called")
            .exists()
    );
}
