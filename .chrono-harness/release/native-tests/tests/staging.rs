#[path = "support/recipe.rs"]
mod fixture;
use chrono_host_release::{recipe::Loaded,staging::Staging};
use serde_json::{Value,json};
use std::{fs,os::unix::fs::{PermissionsExt,symlink}};

fn source(f: &fixture::Fixture) -> Value {
    let path=f.root().join("out/a");fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path,b"the selected program").unwrap();fs::set_permissions(path,fs::Permissions::from_mode(0o751)).unwrap();
    json!({"a":{"identity":{"path":"out/a","sha256":chrono_harness::sha256(b"the selected program"),"size":20,"mode":489,"error":null}}})
}

#[test]
fn only_selected_consumers_receive_exact_bytes_mode_and_three_real_observations() {
    let f=fixture::Fixture::new();let assets=source(&f);let loaded=Loaded::load(f.root()).unwrap();
    let mut staging=Staging::selected(&loaded,&assets).unwrap();
    assert_eq!(staging.destinations(),["consumer/a"]);
    staging.copy(f.root()).unwrap();
    let file=f.root().join("consumer/a");assert_eq!(fs::read(&file).unwrap(),b"the selected program");assert_eq!(fs::metadata(file).unwrap().permissions().mode()&0o7777,0o751);
    assert!(!f.root().join("out/b").exists());assert!(!f.root().join("consumer/b").exists());
    for (phase,operation) in [("staged",None),("before-verification",Some("test.a")),("after-verification",Some("test.a"))] { assert!(staging.observe(f.root(),phase,operation)); }
    let receipt=staging.receipt();let observations=receipt["observations"].as_array().unwrap();
    assert_eq!(observations.len(),3);
    for observation in observations { assert_eq!(observation["matches"],true);assert_eq!(observation["files"].as_array().unwrap().len(),2); }
    assert_eq!(receipt["bindings"][0]["expected"],assets["a"]["identity"]);
}

#[test]
fn source_or_destination_byte_and_mode_drift_records_the_actual_failure() {
    for (path,mode) in [("out/a",false),("out/a",true),("consumer/a",false),("consumer/a",true)] {
        let f=fixture::Fixture::new();let assets=source(&f);let loaded=Loaded::load(f.root()).unwrap();let mut staging=Staging::selected(&loaded,&assets).unwrap();
        staging.copy(f.root()).unwrap();assert!(staging.observe(f.root(),"staged",None));
        if mode { fs::set_permissions(f.root().join(path),fs::Permissions::from_mode(0o755)).unwrap(); } else { fs::write(f.root().join(path),b"changed program").unwrap(); }
        assert!(!staging.observe(f.root(),"after-verification",Some("test.a")),"{path} mode={mode}");
        let receipt=staging.receipt();let file=receipt["observations"][1]["files"].as_array().unwrap().iter().find(|f|f["path"]==path).unwrap();
        assert_eq!(file["error"],Value::Null);
        if mode { assert_eq!(file["mode"],0o755); } else { assert_eq!(file["sha256"],chrono_harness::sha256(b"changed program")); }
        assert_eq!(receipt["bindings"][0]["expected"],assets["a"]["identity"]);
    }
}

#[test]
fn source_drift_or_symlink_never_replaces_the_existing_consumer() {
    for mutation in 0..3 {
        let f=fixture::Fixture::new();let assets=source(&f);let loaded=Loaded::load(f.root()).unwrap();let mut staging=Staging::selected(&loaded,&assets).unwrap();
        fs::create_dir(f.root().join("consumer")).unwrap();let destination=f.root().join("consumer/a");fs::write(&destination,b"existing program").unwrap();
        match mutation {
            0=>fs::write(f.root().join("out/a"),b"changed source").unwrap(),
            1=>{let source=f.root().join("out/a");fs::remove_file(&source).unwrap();symlink(&destination,source).unwrap();},
            2=>{fs::remove_file(&destination).unwrap();symlink(f.root().join("out/a"),&destination).unwrap();},
            _=>unreachable!(),
        }
        assert!(staging.copy(f.root()).is_err());assert!(!staging.observe(f.root(),"staged",None));
        if mutation<2 { assert_eq!(fs::read(destination).unwrap(),b"existing program"); } else { assert!(fs::symlink_metadata(destination).unwrap().file_type().is_symlink()); }
    }
}

#[test]
fn empty_selection_has_empty_successful_observations_and_invalid_asset_vectors_fail() {
    let f=fixture::Fixture::new();let loaded=Loaded::load(f.root()).unwrap();let mut staging=Staging::selected(&loaded,&json!({})).unwrap();
    assert!(staging.destinations().is_empty());staging.copy(f.root()).unwrap();assert!(staging.observe(f.root(),"staged",None));
    assert_eq!(staging.receipt()["observations"][0]["files"],json!([]));
    for value in [json!({"unknown":{}}),json!({"a":{"identity":{}}}),json!([])] { assert!(Staging::selected(&loaded,&value).is_err()); }
}
