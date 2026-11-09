//! 外部 tar 接口样本，独立指定原路径、文件类型、mode 和负载。
use std::{fs::File, path::Path};

pub struct Entry<'a> {
    pub name: &'a [u8],
    pub kind: tar::EntryType,
    pub mode: u32,
    pub bytes: &'a [u8],
}

pub fn write(path: &Path, entries: &[Entry<'_>]) {
    let file = File::create(path).unwrap();
    let mut archive = tar::Builder::new(file);
    for entry in entries {
        let mut header = tar::Header::new_gnu();
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_entry_type(entry.kind);
        header.set_mode(entry.mode);
        header.set_size(entry.bytes.len() as u64);
        // 负向样本需要真实的非法路径字节，不能由库的写入路径校验提前拒绝。
        assert!(entry.name.len() < 100);
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..entry.name.len()].copy_from_slice(entry.name);
        header.set_cksum();
        archive.append(&header, entry.bytes).unwrap();
    }
    archive.finish().unwrap();
}
