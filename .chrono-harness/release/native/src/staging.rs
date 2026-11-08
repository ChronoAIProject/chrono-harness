//! 在登记消费路径暂存已验资产；观察记录真实身份与失败，不代填成功。
use crate::{recipe::Loaded, transport::{self, ExecutableIdentity}};
use serde_json::{Value,json};
use std::{collections::BTreeMap, fs::{self,File}, io::Write, os::unix::fs::PermissionsExt, path::Path};

fn error(message: impl std::fmt::Display) -> String { format!("E_RELEASE_STAGING: {message}") }

struct Binding {
    asset: String,
    source: String,
    destination: String,
    expected: ExecutableIdentity,
}

pub struct Staging {
    release_plan: String,
    host_config: String,
    bindings: Vec<Binding>,
    observations: Vec<Value>,
}

impl Staging {
    /// assets 来自所选 build 原件；只取这些资产登记的消费目的地。
    pub fn selected(loaded: &Loaded, assets: &Value) -> Result<Self,String> {
        let assets = assets.as_object().ok_or_else(|| error("missing selected asset vector"))?;
        if assets.keys().any(|id| !loaded.plan.assets.contains_key(id)) { return Err(error("unknown selected release asset")); }
        let mut bindings=Vec::new();
        for row in &loaded.staging {
            let Some(asset)=assets.get(&row.asset) else { continue; };
            let identity=&asset["identity"];
            if identity["path"] != row.source || identity.get("error") != Some(&Value::Null) { return Err(error("selected source identity differs from release plan")); }
            bindings.push(Binding { asset:row.asset.clone(),source:row.source.clone(),destination:row.destination.clone(),expected:ExecutableIdentity::from_receipt(identity)? });
        }
        Ok(Self { release_plan:loaded.recipe.consumer_staging.release_plan.clone(),host_config:loaded.recipe.consumer_staging.host_config.clone(),bindings,observations:Vec::new() })
    }

    /// 调用方用原 git 过程核这些字面路径未被跟踪，再暂存。
    pub fn destinations(&self) -> Vec<&str> { self.bindings.iter().map(|b| b.destination.as_str()).collect() }

    pub fn copy(&self, root: &Path) -> Result<(),String> {
        for binding in &self.bindings {
            if transport::observe(root,&binding.source)? != binding.expected { return Err(error("selected source changed before staging")); }
            let source=chrono_harness::no_symlink_parents(root,&binding.source)?;
            let destination=chrono_harness::prepare_publication(root,&binding.destination)?;
            let mut temporary=tempfile::NamedTempFile::new_in(destination.parent().ok_or_else(|| error("missing destination parent"))?).map_err(error)?;
            let mut source=File::open(source).map_err(error)?;
            let (sha256,size)=chrono_harness::copy_hashed(&mut source,temporary.as_file_mut())?;
            if sha256 != binding.expected.sha256 || size != binding.expected.size { return Err(error("source bytes changed during staging")); }
            temporary.as_file_mut().flush().map_err(error)?;
            temporary.as_file().set_permissions(fs::Permissions::from_mode(binding.expected.mode)).map_err(error)?;
            chrono_harness::no_symlink_parents(root,&binding.destination)?;
            temporary.persist(destination).map_err(error)?;
        }
        Ok(())
    }

    /// 即使某个路径读取失败，也保留全部可取得的观察；调用方保留原 child 结果。
    pub fn observe(&mut self, root: &Path, phase: &str, operation: Option<&str>) -> bool {
        let mut files=BTreeMap::new();let mut matches=true;
        for binding in &self.bindings {
            for path in [&binding.source,&binding.destination] {
                let observed=files.entry(path.as_str()).or_insert_with(|| match transport::observe(root,path) {
                    Ok(identity)=>identity.receipt(path),
                    Err(failure)=>json!({"path":path,"sha256":null,"size":null,"mode":null,"error":failure}),
                });
                matches &= observed.get("error") == Some(&Value::Null)
                    && observed["sha256"] == binding.expected.sha256 && observed["size"] == binding.expected.size && observed["mode"] == binding.expected.mode;
            }
        }
        self.observations.push(json!({"phase":phase,"operation":operation,"matches":matches,"files":files.values().collect::<Vec<_>>()}));
        matches
    }

    pub fn receipt(&self) -> Value {
        let bindings:Vec<_>=self.bindings.iter().map(|b| json!({"asset":b.asset,"source":b.source,"destination":b.destination,"expected":b.expected.receipt(&b.source)})).collect();
        json!({"release_plan":self.release_plan,"host_config":self.host_config,"bindings":bindings,"observations":self.observations})
    }
}
