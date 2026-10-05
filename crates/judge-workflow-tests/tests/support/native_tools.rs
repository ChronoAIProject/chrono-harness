use super::*;
use std::{os::unix::fs::PermissionsExt, path::Path};

pub(super) fn install(path: &Path) {
    fs::copy(env!("CARGO_BIN_EXE_chrono-workflow-test-tool"), path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

pub(super) fn bind(h: &mut Host, variable: &str, program: &Path, consumers: &[&str]) {
    let node = format!("environment:{variable}");
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["environment"]["values"][variable] = json!(program);
    for consumer in consumers {
        let binding = cfg["input_closure"]["bindings"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|binding| binding["consumer"] == *consumer)
            .expect("explicit native tool consumer binding");
        let inputs = binding["inputs"].as_array_mut().unwrap();
        if !inputs.iter().any(|input| input == &node) {
            inputs.push(json!(node));
        }
    }
    let edges = h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap();
    for consumer in consumers {
        let declaration = edge(&node, "runtime-input", consumer);
        if !edges.contains(&declaration) {
            edges.push(declaration);
        }
    }
}
