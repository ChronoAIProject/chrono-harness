//! 只采集已登记失败目录和原生失败标记；两者共用有限内容容量。
use chrono_harness::{no_symlink_parents, relative_path};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub const BOUND: u64 = 64 * 1024 * 1024;

pub struct Retained {
    pub nested: Option<Value>,
    pub registered: Option<Value>,
    pub failures: Vec<String>,
}

struct Collector<'a> {
    output: &'a Path,
    used: u64,
    failures: Vec<String>,
}

impl Collector<'_> {
    fn file(&mut self, root: &Path, file: &Path, destination: &str) -> Value {
        let relative = match file.strip_prefix(root).ok().and_then(Path::to_str) {
            Some(relative) => relative,
            None => {
                self.failures
                    .push("failure evidence has no root-relative UTF-8 path".into());
                return json!({"status":"unavailable","reason":"invalid retained path"});
            }
        };
        let mut record = json!({"source":relative,"status":"omitted"});
        let result = (|| -> Result<(), String> {
            let metadata = fs::symlink_metadata(file).map_err(|e| e.to_string())?;
            if !metadata.is_file() || no_symlink_parents(root, relative).is_err() {
                record["reason"] = json!("not a regular retained file");
                return Ok(());
            }
            let remaining = BOUND - self.used;
            if metadata.len() > remaining {
                record["reason"] = json!("bounded evidence limit");
                record["bytes"] = json!(metadata.len());
                return Ok(());
            }
            let mut input = File::open(file).map_err(|e| e.to_string())?;
            let mut raw = Vec::new();
            (&mut input)
                .take(remaining + 1)
                .read_to_end(&mut raw)
                .map_err(|e| e.to_string())?;
            let after = input.metadata().map_err(|e| e.to_string())?;
            if raw.len() as u64 != metadata.len()
                || (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.len(),
                    metadata.mtime(),
                    metadata.mtime_nsec(),
                ) != (
                    after.dev(),
                    after.ino(),
                    after.len(),
                    after.mtime(),
                    after.mtime_nsec(),
                )
            {
                record["reason"] = json!("source changed during retention");
                record["bytes"] = json!(raw.len());
                return Ok(());
            }
            relative_path(destination)?;
            let target = chrono_harness::prepare_publication(self.output, destination)?;
            let mut target = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|e| e.to_string())?;
            target
                .write_all(&raw)
                .and_then(|()| target.flush())
                .map_err(|e| e.to_string())?;
            self.used += raw.len() as u64;
            record = json!({"source":relative,"status":"retained","path":destination,"bytes":raw.len(),"sha256":chrono_harness::sha256(&raw)});
            Ok(())
        })();
        if let Err(failure) = result {
            record["status"] = json!("unavailable");
            record["reason"] = json!(&failure);
            self.failures.push(failure);
        }
        record
    }

    fn tree(&mut self, root: &Path, start: &Path, prefix: &str) -> Vec<Value> {
        let mut pending = vec![start.to_owned()];
        let mut records = Vec::new();
        while let Some(current) = pending.pop() {
            let relative = match current.strip_prefix(root).ok().and_then(Path::to_str) {
                Some(relative) => relative,
                None => {
                    self.failures.push("non UTF-8 failure evidence path".into());
                    records.push(json!({"source":current.strip_prefix(root).unwrap_or(&current).to_string_lossy(),"status":"unavailable","reason":"non UTF-8 failure evidence path"}));
                    continue;
                }
            };
            if let Err(failure) = no_symlink_parents(root, relative) {
                self.failures.push(failure.clone());
                records.push(json!({"source":relative,"status":"unavailable","reason":failure}));
                continue;
            }
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.is_dir() => {
                    match fs::read_dir(&current).and_then(|entries| {
                        entries
                            .map(|e| e.map(|e| e.path()))
                            .collect::<Result<Vec<_>, _>>()
                    }) {
                        Ok(mut entries) => {
                            entries.sort();
                            pending.extend(entries.into_iter().rev());
                        }
                        Err(failure) => {
                            self.failures.push(failure.to_string());
                            records.push(json!({"source":relative,"status":"unavailable","reason":failure.to_string()}));
                        }
                    }
                }
                Ok(_) => records.push(self.file(root, &current, &format!("{prefix}/{relative}"))),
                Err(failure) => {
                    self.failures.push(failure.to_string());
                    records.push(json!({"status":"unavailable","reason":failure.to_string()}));
                }
            }
        }
        records
    }
}

fn status(files: &[Value]) -> &'static str {
    if files.iter().all(|f| f["status"] == "retained") {
        "retained"
    } else {
        "partial"
    }
}

/// 非 UTF-8 业务输出仍是原字节；仅标记中的路径要求 UTF-8。
pub fn retain(
    root: &Path,
    output: &Path,
    phase: &str,
    directories: &[String],
    stdout: &[u8],
    stderr: &[u8],
) -> Retained {
    let mut collector = Collector {
        output,
        used: 0,
        failures: Vec::new(),
    };
    let marker = b"original native publication process evidence: ";
    let source = stdout
        .split(|b| *b == b'\n')
        .chain(stderr.split(|b| *b == b'\n'))
        .find_map(|line| {
            line.windows(marker.len())
                .position(|v| v == marker)
                .map(|index| &line[index + marker.len()..])
        });
    let nested=source.map(|source| {
        let result=(|| -> Result<PathBuf,String> {
            let source=PathBuf::from(std::str::from_utf8(source).map_err(|e|e.to_string())?.trim());
            let metadata=fs::symlink_metadata(&source).map_err(|e|e.to_string())?;
            let temp=fs::canonicalize(std::env::temp_dir()).map_err(|e|e.to_string())?;
            let resolved=fs::canonicalize(&source).map_err(|e|e.to_string())?;
            if !source.is_absolute() || !metadata.is_dir() || resolved.parent()!=Some(temp.as_path()) || !resolved.file_name().and_then(|s|s.to_str()).is_some_and(|s|s.starts_with("chrono-native-publication-failure-")) {return Err("retained directory is outside the native fixture boundary".into());}
            Ok(resolved)
        })();
        match result {
            Ok(source)=>{
                let mut files=Vec::new();let prefix=format!("failure-evidence/{phase}");
                for name in ["stdout","stderr","process.json"] {files.push(collector.file(&source,&source.join(name),&format!("{prefix}/{name}")));}
                let preparation=source.join("state/collection/preparation");
                match fs::symlink_metadata(&preparation) {
                    Ok(metadata) if metadata.is_dir()=>files.extend(collector.tree(&source,&preparation,&prefix)),
                    Ok(_)=>files.push(json!({"source":"state/collection/preparation","status":"omitted","reason":"not a regular retained directory"})),
                    Err(failure) if failure.kind()==std::io::ErrorKind::NotFound=>{},
                    Err(failure)=>{collector.failures.push(failure.to_string());files.push(json!({"source":"state/collection/preparation","status":"unavailable","reason":failure.to_string()}));}
                }
                json!({"status":status(&files),"source":source.file_name().and_then(|s|s.to_str()),"files":files,"bytes":collector.used,"bound_bytes":BOUND})
            }
            Err(failure)=>{collector.failures.push(failure.clone());json!({"status":"unavailable","reason":failure})}
        }
    });
    let registered = if directories.is_empty() {
        None
    } else {
        let before = collector.used;
        let mut sources = Vec::new();
        let mut files = Vec::new();
        for directory in directories {
            let relative = directory.trim_end_matches('/');
            match no_symlink_parents(root,relative).and_then(|path| match fs::symlink_metadata(&path) {
                Ok(metadata)=>Ok(Some((path,metadata))),
                Err(failure) if failure.kind()==std::io::ErrorKind::NotFound=>Ok(None),
                Err(failure)=>Err(failure.to_string()),
            }) {
                Ok(None)=>sources.push(json!({"source":directory,"status":"absent"})),
                Ok(Some((path,metadata))) if metadata.is_dir()=>{
                    sources.push(json!({"source":directory,"status":"observed"}));files.extend(collector.tree(root,&path,&format!("failure-evidence/registered/{phase}")));
                }
                Ok(Some(_))=>sources.push(json!({"source":directory,"status":"unavailable","reason":"not a regular directory"})),
                Err(failure)=>{collector.failures.push(failure.clone());sources.push(json!({"source":directory,"status":"unavailable","reason":failure}));}
            }
        }
        let complete = sources
            .iter()
            .all(|s| s["status"] == "observed" || s["status"] == "absent")
            && status(&files) == "retained";
        Some(
            json!({"status":if complete {"retained"} else {"partial"},"sources":sources,"files":files,"bytes":collector.used-before,"bound_bytes":BOUND-before}),
        )
    };
    Retained {
        nested,
        registered,
        failures: collector.failures,
    }
}
