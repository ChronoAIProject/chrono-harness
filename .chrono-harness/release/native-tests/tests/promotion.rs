#[path = "support/recipe.rs"]
mod fixture;
use chrono_host_release::{
    delivery, execution::Environment, identity::{Channel, Identity, VerifiedCi},
    promotion, publication::{Commands, Reply}, recipe::Loaded,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

fn stable() -> Identity {
    Identity {channel: Channel::Stable, version: "v1.2.3".into(), source_commit: "a".repeat(40), source_tree: "b".repeat(40), verified_ci: Some(VerifiedCi {run_id:10, attempt:1})}
}
fn preview() -> Identity {
    Identity {channel:Channel::Preview, version:format!("v1.2.3-dev.10.1.g{}", "a".repeat(40)), ..stable()}
}
fn run() -> Value {
    json!({"id":20,"run_attempt":2,"workflow_id":100,"path":".github/workflows/release.yml","repository":{"full_name":"example/product"},"status":"completed","conclusion":"success","event":"workflow_run","head_sha":"c".repeat(40)})
}
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for row in fs::read_dir(from).unwrap() {
        let row=row.unwrap(); let target=to.join(row.file_name());
        if row.file_type().unwrap().is_dir() {copy(&row.path(), &target)} else {fs::copy(row.path(), target).unwrap();}
    }
}
struct Mock {
    originals: PathBuf,
    run: Value,
    release: Value,
    assets: Vec<Value>,
    bad: &'static str,
    calls: Vec<(String, Vec<String>)>,
}
impl Commands for Mock {
    fn body(&mut self, _: &Value) -> Result<String,String> {panic!("promotion must not mutate GitHub")}
    fn execute(&mut self, program:&str, args:&[String]) -> Result<Reply,String> {
        self.calls.push((program.into(),args.to_vec()));
        let reply=|v:Value|Ok(Reply {exit:0,stdout:serde_json::to_vec(&v).unwrap()});
        if program=="git" {
            assert_eq!(&args[..4], ["--no-replace-objects","rev-parse","--verify","--end-of-options"]);
            assert!(args[4].ends_with(":.github/workflows/release.yml"));
            return Ok(Reply {exit:0,stdout:format!("{}\n",if self.bad=="workflow" && args[4].starts_with('a') {"e".repeat(40)} else {"d".repeat(40)}).into_bytes()});
        }
        assert_eq!(program,"gh");
        if args[0]=="run" {
            assert_eq!(&args[..5], ["run","download","20","--repo","example/product"]);
            assert_eq!(args[5],"--name"); assert_eq!(args[7],"--dir");assert_eq!(args.len(),9);
            if self.bad=="download" {return Ok(Reply {exit:17,stdout:b"original download failed".to_vec()});}
            let source=if args[6]=="publication-20-2" {"publication"} else {assert_eq!(args[6],"package-20-1");"package"};
            copy(&self.originals.join(source),Path::new(&args[8]));
            return Ok(Reply {exit:0,stdout:vec![]});
        }
        assert_eq!(&args[..3],["api","--method","GET"]);
        let path=&args[3];
        if path.ends_with("/actions/runs/20/attempts/2") {return reply(self.run.clone())}
        if path.contains("/artifacts?") {
            let name=if path.contains("name=publication-") {"publication-20-2"} else {assert!(path.contains("name=package-20-1"));"package-20-1"};
            return reply(json!({"total_count":1,"artifacts":[{"id":if name.starts_with("publication") {32} else if self.bad=="artifact" {31} else {30},"name":name,"expired":self.bad=="expired","size_in_bytes":123,"workflow_run":{"id":20,"head_sha":"c".repeat(40)}}]}));
        }
        if path.ends_with("/actions/runs/10/attempts/1") {
            return reply(json!({"id":10,"run_attempt":1,"workflow_id":99,"path":".github/workflows/ci.yml","event":"push","head_branch":"dev","head_sha":if self.bad=="ci-source" {"e".repeat(40)} else {"a".repeat(40)},"status":"completed","conclusion":"success","repository":{"full_name":"example/product"}}));
        }
        if path.contains("/releases/tags/") {return reply(self.release.clone())}
        if path.ends_with("/releases/7/assets?per_page=100") {return reply(json!(self.assets))}
        if path.contains("/git/ref/tags/") {return reply(json!({"object":{"type":"commit","sha":if self.bad=="tag" {"e".repeat(40)} else {"a".repeat(40)}}))}
        panic!("unexpected {program} {args:?}");
    }
}

fn setup() -> (fixture::Fixture, Loaded, delivery::Config, Environment, Mock) {
    let mut f=fixture::Fixture::new();
    f.plan["version"]=json!("v1.2.3");f.save();
    let loaded=Loaded::load(f.root()).unwrap();
    let config:delivery::Config=serde_json::from_value(json!({
        "schema":"chrono-release-delivery/v1","publication":{"repository":"example/product","workflow_id":99,"workflow_path":".github/workflows/ci.yml","branch":"dev"},
        "frontend":"fixture","packages":{"fixture":{"platform":"linux-x86_64","directory":"unused"}},"tools":{"git":"git","gh":"gh"},"curl":"/usr/bin/curl",
        "installation":{"publisher":"publish","directory":"publication/","shell":"/bin/sh"},"promotion":{"workflow_id":100,"workflow_config":"workflow.json"}
    })).unwrap();
    fixture::write(f.root(),"workflow.json",&json!({"workflow_path":".github/workflows/release.yml","delivery":{"publisher":"publish"},"jobs":[{"id":"publish","artifact_name":"publication"},{"id":"native_collect","artifact_name":"package"}]}));
    let originals=f.root().join("originals"); let package=originals.join("package");let publication=originals.join("publication");
    fs::create_dir_all(publication.join("release")).unwrap();fs::create_dir_all(&package).unwrap();
    let mut tools=BTreeMap::new();let mut observed=BTreeMap::new();
    for name in ["a","b"] {
        let file=format!("{name}-linux-x86_64");let bytes=format!("original tool {name}").into_bytes();
        for dest in [&package, &publication.join("release")] {fs::write(dest.join(&file),&bytes).unwrap();}
        let sha=chrono_harness::sha256(&bytes);
        tools.insert(name,json!({"file":file,"sha256":sha,"size":bytes.len()}));
        observed.insert(name,json!({"path":file,"sha256":sha,"size":bytes.len(),"mode":493}));
    }
    let manifest=json!({"schema":"chrono-release/v1","version":preview().version,"source_commit":stable().source_commit,"source_tree":stable().source_tree,"platforms":{"linux-x86_64":tools}});
    for dest in [&package, &publication.join("release")] {fixture::write(dest,"release.json",&manifest);}
    let mut retained=BTreeMap::new();
    for id in ["build_a","build_b","test_none","test_a"] {
        let dir=package.join("originals").join(id);fs::create_dir_all(&dir).unwrap();fs::write(dir.join("receipt.json"),b"original receipt").unwrap();
        retained.insert(id,json!({"receipt.json":{"path":"receipt.json","sha256":chrono_harness::sha256(b"original receipt"),"size":16}}));
    }
    let empty=json!({"path":"empty","sha256":chrono_harness::sha256(b""),"size":0});
    for dest in [&package,&publication] {fs::write(dest.join("empty"),b"").unwrap();}
    fixture::write(&package,"receipt.json",&json!({"schema":"chrono-release-unit/v1","status":"passed","failure":null,"started_ns":1,"finished_ns":2,"platform_started_ns":1,"platform_span_ns":1,
        "unit":loaded.recipe.collector,"lineage":{"kind":"github","run_id":"20","attempt":"1","job":"native_collect"},
        "context":{"source_commit":preview().source_commit,"source_tree":preview().source_tree,"release_identity":preview()},"package":{"release_sha256":chrono_harness::file_identity(&package.join("release.json")).unwrap().0,"assets":observed},"originals":retained,"processes":[{"exit_code":0,"stdout":empty,"stderr":empty}]}));
    fs::write(publication.join("release/install.sh"),b"original launcher").unwrap();
    let mut assets=BTreeMap::new(); let mut remote=Vec::new();
    for name in ["release.json","install.sh","a-linux-x86_64","b-linux-x86_64"] {
        let (sha,size)=chrono_harness::file_identity(&publication.join("release").join(name)).unwrap();
        assets.insert(name,json!({"path":format!("old-output/{name}"),"sha256":sha,"size":size}));
        remote.push(json!({"name":name,"digest":format!("sha256:{sha}"),"size":size,"state":"uploaded"}));
    }
    let release=json!({"id":7,"tag_name":preview().version,"target_commitish":stable().source_commit,"draft":false,"prerelease":true});
    fixture::write(&publication,"receipt.json",&json!({"schema":"chrono-release-publication/v1","status":"passed","identity":preview(),
        "lineage":{"kind":"github","run_id":"20","attempt":"2","job":"publish"},"dependencies":{"native_collect":{"result":"success","outputs":{"artifact_id":"30","run_id":"20","attempt":"1"}}},
        "processes":[{"exit_code":0,"stdout":empty,"stderr":empty,"observation":{"failure":null,"exit_signal":null,"engine_exit_code":0}}],"result":{"release":release,"assets":assets}}));
    let environment=Environment::from([
        ("CHRONO_RELEASE_CHANNEL".into(),"stable".into()),("CHRONO_RELEASE_SOURCE".into(),stable().source_commit.into()),
        ("CHRONO_RELEASE_CI_RUN_ID".into(),"10".into()),("CHRONO_RELEASE_CI_ATTEMPT".into(),"1".into()),("CHRONO_RELEASE_VERSION".into(),"v1.2.3".into()),
        ("CHRONO_RELEASE_RUN".into(),"40".into()),("CHRONO_RELEASE_PREVIEW_RUN".into(),"20".into()),("CHRONO_RELEASE_PREVIEW_ATTEMPT".into(),"2".into()),
    ]);
    let mock=Mock {originals,run:run(),release,assets:remote,bad:"none",calls:vec![]};
    (f,loaded,config,environment,mock)
}

#[test]
fn stable_promotion_preserves_preview_identity_and_actual_producing_attempt() {
    let (f,loaded,config,environment,mut mock)=setup();let output=f.root().join("out");fs::create_dir(&output).unwrap();
    let original=fs::read(mock.originals.join("package/receipt.json")).unwrap();
    let prepared=promotion::prepare(f.root(),&output,&config,&loaded,&stable(),&environment,&mut mock).unwrap();
    assert_eq!(prepared.identity,preview());assert_eq!(prepared.selections["fixture"].attempt,"1");
    let package=&prepared.packages["fixture"];
    delivery::verify_package(f.root(),&loaded,package,&prepared.selections["fixture"],&preview()).unwrap();
    assert_eq!(fs::read(f.root().join(&package.directory).join("receipt.json")).unwrap(),original);
    assert_eq!(prepared.observation["run"]["run_attempt"],2);
    assert_eq!(prepared.observation["workflow"]["source_commit"],stable().source_commit);
    assert_eq!(prepared.observation["workflow"]["workflow_commit"],"c".repeat(40));
    assert_eq!(prepared.observation["package_artifacts"]["fixture"]["id"],30);
}

#[test]
fn promotion_rejects_failed_or_unbound_origins_without_mutation_or_rebuild() {
    for bad in ["run","attempt","workflow","expired","artifact","download","identity","process","bytes","dependency","public-assets","draft","tag","ci-source","missing-input"] {
        let (f,loaded,config,mut environment,mut mock)=setup();mock.bad=bad;
        let receipt_path=mock.originals.join("publication/receipt.json");let mut receipt:Value=serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        match bad {
            "run"=>mock.run["conclusion"]=json!("failure"),"attempt"=>mock.run["run_attempt"]=json!(1),
            "identity"=>receipt["identity"]["source_tree"]=json!("e".repeat(40)),
            "process"=>receipt["processes"][0]["observation"]["engine_exit_code"]=json!(17),
            "bytes"=>fs::write(mock.originals.join("publication/release/a-linux-x86_64"),b"corrupt").unwrap(),
            "dependency"=>receipt["dependencies"]["native_collect"]["outputs"]["attempt"]=json!("3"),
            "public-assets"=>mock.assets[0]["digest"]=json!(format!("sha256:{}","e".repeat(64))),
            "draft"=>mock.release["draft"]=json!(true),
            "missing-input"=>{environment.remove(std::ffi::OsStr::new("CHRONO_RELEASE_PREVIEW_RUN"));},
            _=>{},
        }
        fixture::write(mock.originals.join("publication").as_path(),"receipt.json",&receipt);
        let output=f.root().join("out");fs::create_dir(&output).unwrap();
        let error=promotion::prepare(f.root(),&output,&config,&loaded,&stable(),&environment,&mut mock).err().unwrap_or_else(||panic!("accepted {bad}"));
        assert!(!error.is_empty(),"{bad}");
        assert!(mock.calls.iter().all(|(p,a)|p=="git" || (p=="gh" && (a[0]=="run" || a[2]=="GET"))),"{bad}: {:?}",mock.calls);
    }
}
