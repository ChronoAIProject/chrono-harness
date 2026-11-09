//! 显式转换仍被实际消费者保管的旧分页登记；普通运行不读取旧形状。
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousOwner {
    schema: String,
    producer: String,
    directory_id: String,
    records_id: String,
    record_layout: String,
}

// These superseded fields remain in each retained original. They do not grant
// new whole-directory ownership, release references, or become runtime policy.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[allow(dead_code)] // Decode the retained custody contract; only its exclusion addresses are executed.
struct PreviousCustody {
    captures: BTreeMap<String, Value>,
    copy_intents: BTreeMap<String, u64>,
    original_release: Option<String>,
    shared_originals: BTreeMap<String, u64>,
    retiring: bool,
    metadata_directory: Option<String>,
    metadata_identity: Option<String>,
    born_nanos: u64,
    disposable_directory: bool,
    directory_custody: Option<Receipt>,
}

struct PreviousRecord {
    name: String,
    bytes: Vec<u8>,
    identity: String,
    object: Object,
    custody: PreviousCustody,
}

fn regular(path: &Path) -> Result<Vec<u8>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("producer partition requires physical regular originals".into());
    }
    fs::read(path).map_err(|e| e.to_string())
}

fn keep(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) if regular(path)? == bytes => Ok(()),
        Ok(_) => Err("producer partition cannot replace an unknown registration".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => write_bytes(path, bytes, true),
        Err(e) => Err(e.to_string()),
    }
}

fn bucket(name: &str) -> bool {
    name.len() == 1
        && name
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn snapshot(
    records: &Path,
    archive: &Path,
    owner_sha: &str,
) -> Result<(BTreeMap<String, String>, Vec<PreviousRecord>), String> {
    let mut buckets = BTreeMap::new();
    let mut result = Vec::new();
    for parent in [records, archive] {
        for entry in fs::read_dir(parent).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "producer partition name is not UTF8")?;
            if !bucket(&name) {
                if parent == records
                    && name.strip_suffix(".json").is_some_and(|name| {
                        name.len() == 64
                            && name
                                .bytes()
                                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    })
                {
                    continue; // Exact prepared current registrations are checked before activation.
                }
                if parent == archive && matches!(name.as_str(), "binding.json" | "owner.json") {
                    continue;
                }
                return Err("producer partition has unknown metadata; preserve originals".into());
            }
            let path = no_symlink_parents(parent, &name)?;
            if buckets.insert(name.clone(), directory_id(&path)?).is_some() {
                return Err("producer partition source bucket has two attachments".into());
            }
            for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let physical = no_symlink_parents(
                    &path,
                    entry
                        .file_name()
                        .to_str()
                        .ok_or("producer partition name is not UTF8")?,
                )?;
                let bytes = regular(&physical)?;
                let mut record: ObjectRecord<Value> = decode(&bytes)?;
                if record.schema != "chrono-retained-producer-object/v1"
                    || record.owner_sha256 != owner_sha
                {
                    return Err("producer partition original owner binding differs".into());
                }
                let object = record
                    .object
                    .as_object_mut()
                    .ok_or("producer partition object is not a map")?;
                let mut custody = serde_json::Map::new();
                for key in [
                    "captures",
                    "copy_intents",
                    "original_release",
                    "shared_originals",
                    "retiring",
                    "metadata_directory",
                    "metadata_identity",
                    "born_nanos",
                    "disposable_directory",
                    "directory_custody",
                ] {
                    if let Some(value) = object.remove(key) {
                        custody.insert(key.into(), value);
                    }
                }
                let custody: PreviousCustody =
                    serde_json::from_value(Value::Object(custody)).map_err(|e| e.to_string())?;
                let object: Object =
                    serde_json::from_value(record.object).map_err(|e| e.to_string())?;
                let current_name = Store::record_name(&object.path)?;
                let file = Path::new(&current_name)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                if file.as_bytes()[0] != name.as_bytes()[0]
                    || entry.file_name() != file
                    || object.rolling.is_some()
                    || (object.consumers.is_empty() && object.cache_lease.is_none())
                {
                    return Err("producer partition original address differs".into());
                }
                result.push(PreviousRecord {
                    name: format!("{name}/{file}"),
                    identity: fingerprint(&physical)?,
                    bytes,
                    object,
                    custody,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((buckets, result))
}

pub(super) fn partition(
    directory: &Path,
    producer: &str,
    candidate: &str,
    bytes: &[u8],
    mut observer: impl FnMut(&str) -> Result<(), String>,
) -> Result<Value, String> {
    let previous: PreviousOwner = decode(bytes)?;
    let records = no_symlink_parents(directory, "producer-records")?;
    if previous.schema != "chrono-retained-producer/v2"
        || previous.producer != producer
        || previous.record_layout != "sha256-nibble/v1"
        || previous.directory_id != directory_id(directory)?
        || previous.records_id != directory_id(&records)?
    {
        return Err("producer partition input identity differs; preserve originals".into());
    }
    let gate = DirectoryGate::available(directory)?
        .ok_or("producer partition has live namespace writers")?;
    let owner_path = no_symlink_parents(directory, "objects.json")?;
    let owner_id = fingerprint(&owner_path)?;
    let archive_name = format!("producer-partition-source-{}", sha256(bytes));
    let archive = no_symlink_parents(directory, &archive_name)?;
    if !archive.exists() {
        let prepared = tempfile::Builder::new()
            .prefix("producer-partition-prepare-")
            .tempdir_in(directory)
            .map_err(|e| e.to_string())?;
        let binding = value!({"schema":"chrono-producer-layout-source/v1","owner_sha256":sha256(bytes),
            "records_id":previous.records_id,"directory_id":directory_id(prepared.path())?});
        write_json(&prepared.path().join("binding.json"), &binding, true)?;
        write_bytes(&prepared.path().join("owner.json"), bytes, true)?;
        fs::rename(prepared.path(), &archive).map_err(|e| e.to_string())?;
    }
    let binding = value!({"schema":"chrono-producer-layout-source/v1","owner_sha256":sha256(bytes),
        "records_id":previous.records_id,"directory_id":directory_id(&archive)?});
    if json(&regular(&archive.join("binding.json"))?)? != binding
        || regular(&archive.join("owner.json"))? != bytes
    {
        return Err("producer partition original archive changed".into());
    }
    let archive_id = directory_id(&archive)?;
    let (buckets, originals) = snapshot(&records, &archive, &sha256(bytes))?;
    let mut directories = Vec::new();
    let mut leases = Vec::new();
    for original in &originals {
        let object = &original.object;
        match (
            &original.custody.metadata_directory,
            &original.custody.metadata_identity,
        ) {
            (Some(path), Some(identity)) => {
                if path != &format!("producer-metadata/{}", sha256(object.path.as_bytes())) {
                    return Err("producer partition original metadata address differs".into());
                }
                let path = no_symlink_parents(directory, path)?;
                if directory_id(&path)? != *identity {
                    return Err("producer partition original metadata identity differs".into());
                }
                directories.push(
                    DirectoryGate::available(&path)?
                        .ok_or("producer partition has live object metadata writers")?,
                );
            }
            (None, None) => (),
            _ => return Err("producer partition original metadata binding is partial".into()),
        }
        let allocation = no_symlink_parents(directory, &object.path)?;
        match fs::symlink_metadata(&allocation) {
            Ok(_) => {
                if directory_id(&allocation)? != object.identity {
                    return Err("producer partition original allocation identity differs".into());
                }
                directories.push(
                    DirectoryGate::available(&allocation)?
                        .ok_or("producer partition has live object metadata writers")?,
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        if let Some(lease) = &object.cache_lease {
            leases.push(
                Lease::acquire(
                    &no_symlink_parents(directory, &lease.path)?,
                    Some(&lease.id),
                    false,
                    true,
                    None,
                )?
                .ok_or("producer partition has live allocation consumers")?,
            );
        }
    }
    let stable = || -> Result<(), String> {
        gate.stable()?;
        if fingerprint(&owner_path)? != owner_id
            || regular(&owner_path)? != bytes
            || directory_id(directory)? != previous.directory_id
            || directory_id(&records)? != previous.records_id
            || directory_id(&archive)? != archive_id
            || json(&regular(&archive.join("binding.json"))?)? != binding
            || regular(&archive.join("owner.json"))? != bytes
        {
            return Err("producer partition original owner changed".into());
        }
        for d in &directories {
            d.stable()?;
        }
        for lease in &leases {
            lease.stable()?;
        }
        for original in &originals {
            Store::stable_in(
                directory,
                producer,
                &previous.directory_id,
                &original.object,
            )?;
        }
        let (current_buckets, current) = snapshot(&records, &archive, &sha256(bytes))?;
        if current_buckets != buckets
            || current.len() != originals.len()
            || current
                .iter()
                .zip(&originals)
                .any(|(a, b)| a.name != b.name || a.identity != b.identity || a.bytes != b.bytes)
        {
            return Err("producer partition source changed during exclusion".into());
        }
        Ok(())
    };
    stable()?;
    let owner = Owner {
        schema: "chrono-retained-producer/v2".into(),
        producer: producer.into(),
        directory_id: previous.directory_id.clone(),
        records_id: previous.records_id.clone(),
    };
    let mut owner_bytes = serde_json::to_vec_pretty(&owner).map_err(|e| e.to_string())?;
    owner_bytes.push(b'\n');
    let mut declarations = Vec::new();
    let mut names = BTreeSet::new();
    let mut prepared = BTreeMap::new();
    for original in &originals {
        let name = Store::record_name(&original.object.path)?;
        names.insert(Path::new(&name).file_name().unwrap().to_owned());
        let next = ObjectRecord {
            schema: "chrono-retained-producer-object/v1".into(),
            owner_sha256: sha256(&owner_bytes),
            object: &original.object,
        };
        let next_bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
        keep(&no_symlink_parents(directory, &name)?, &next_bytes)?;
        prepared.insert(
            Path::new(&name).file_name().unwrap().to_owned(),
            next_bytes.clone(),
        );
        declarations.push(value!({"path":name,"sha256":sha256(&next_bytes),
            "original":{"path":format!("{archive_name}/{}",original.name),"sha256":sha256(&original.bytes),
                "length":original.bytes.len(),"identity":original.identity}}));
    }
    let prepared_stable = || -> Result<(), String> {
        let mut seen = BTreeSet::new();
        for entry in fs::read_dir(&records).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name().to_str().is_some_and(bucket) {
                continue;
            }
            let file = entry.file_name();
            let expected = prepared
                .get(&file)
                .ok_or("producer partition has unknown prepared registrations")?;
            if regular(&no_symlink_parents(
                &records,
                file.to_str().ok_or("producer partition name is not UTF8")?,
            )?)? != *expected
            {
                return Err("producer partition prepared registration changed".into());
            }
            seen.insert(file);
        }
        if seen != names {
            return Err("producer partition prepared registration disappeared".into());
        }
        Ok(())
    };
    prepared_stable()?;
    observer("paged-records-published")?;
    stable()?;
    prepared_stable()?;
    for name in buckets.keys() {
        let path = no_symlink_parents(&records, name)?;
        if path.exists() {
            fs::rename(&path, no_symlink_parents(&archive, name)?).map_err(|e| e.to_string())?;
        }
    }
    observer("paged-buckets-archived")?;
    stable()?;
    prepared_stable()?;
    let record = value!({"schema":"chrono-producer-layout-partition/v1","producer":producer,"candidate":candidate,
        "source":{"path":format!("{archive_name}/owner.json"),"sha256":sha256(bytes),"length":bytes.len(),
            "identity":fingerprint(&archive.join("owner.json"))?,"original_identity":owner_id},
        "archive":binding,"registrations":declarations,"owner_sha256":sha256(&owner_bytes),
        "historical_custody":"retained-in-originals; no additional deletion eligibility",
        "historical_outcomes":"unchanged","disposal":"not-performed","task_completion":"not-claimed"});
    let receipt = Store::receipt_in(directory, "producer-partition", &record)?;
    stable()?;
    prepared_stable()?;
    write_bytes(&owner_path, &owner_bytes, false)?;
    Ok(
        value!({"status":"partitioned","producer":producer,"receipt":receipt,
        "objects":originals.len(),"disposal":"not-performed","historical_outcomes":"unchanged"}),
    )
}
