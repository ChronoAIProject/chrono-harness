//! 编译器源码校验记录的字节语法及实际观察准入。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "verified_identifiers.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub struct SourceChecksum {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: Vec<u8>,
}

pub open spec fn prefix(bytes: Seq<u8>, expected: Seq<u8>) -> bool {
    bytes.len() >= expected.len() && bytes.take(expected.len() as int) == expected
}

pub fn has_prefix(bytes: &[u8], expected: &[u8]) -> (matches: bool)
    ensures matches == prefix(bytes@, expected@),
{
    if bytes.len() < expected.len() { return false; }
    identifiers::equal_bytes(&bytes[..expected.len()], expected)
}

pub open spec fn decimal(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && (bytes.len() == 1 || bytes[0] != 48)
        && forall|i: int| 0 <= i < bytes.len() ==> 48 <= #[trigger] bytes[i] <= 57
}

pub open spec fn source_path(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && forall|i: int| 0 <= i < bytes.len() ==>
        #[trigger] bytes[i] != 0 && bytes[i] != 10 && bytes[i] != 13
}

/// 长度保持编译器原十进制字节；与真实文件长度格式化的结果精确比较，
/// 不把不受信输入转换为机器整数，也不截断或回绕。
pub open spec fn checksum_line(line: Seq<u8>, record: &SourceChecksum) -> bool {
    identifiers::digest_format(record.sha256@) && decimal(record.length@)
        && source_path(record.path@)
        && line == b"# checksum:sha256="@ + record.sha256@
            + b" file_len:"@ + record.length@ + seq![32u8] + record.path@
}

pub open spec fn length_end(line: Seq<u8>, end: int) -> bool {
    93 <= end < line.len() - 1 && line[end] == 32
        && decimal(line.subrange(92, end))
}

pub open spec fn valid_checksum_line(line: Seq<u8>) -> bool {
    line.len() >= 95 && prefix(line, b"# checksum:sha256="@)
        && identifiers::digest_format(line.subrange(18, 82))
        && line.subrange(82, 92) == b" file_len:"@
        && exists|end: int| length_end(line, end)
            && source_path(line.subrange(end + 1, line.len() as int))
}

pub proof fn unique_length_end(line: Seq<u8>, first: int, second: int)
    requires length_end(line, first), length_end(line, second),
    ensures first == second,
{
    if first < second {
        assert(48 <= line.subrange(92, second)[first - 92] <= 57);
    } else if second < first {
        assert(48 <= line.subrange(92, first)[second - 92] <= 57);
    }
}

pub fn copy_bytes(bytes: &[u8]) -> (out: Vec<u8>)
    ensures out@ == bytes@,
{
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), out@ == bytes@.take(i as int),
        decreases bytes.len() - i,
    {
        out.push(bytes[i]);
        assert(bytes@.take(i + 1) =~= bytes@.take(i as int).push(bytes[i as int]));
        i += 1;
    }
    out
}

pub fn parse_checksum_line(line: &[u8]) -> (out: Result<SourceChecksum, String>)
    ensures out.is_ok() == valid_checksum_line(line@),
        out.is_ok() ==> checksum_line(line@, &out.unwrap()),
{
    if line.len() < 95 || !has_prefix(line, b"# checksum:sha256=") {
        return Err("E_VERUS_SOURCES: malformed checksum record".to_string());
    }
    if !identifiers::valid_digest(&line[18..82])
        || !identifiers::equal_bytes(&line[82..92], b" file_len:") {
        return Err("E_VERUS_SOURCES: malformed checksum record".to_string());
    }
    let mut end = 92;
    while end < line.len() && line[end] != 32
        invariant 92 <= end <= line.len(), line.len() >= 95,
            forall|i: int| 92 <= i < end ==> 48 <= #[trigger] line[i] <= 57,
            forall|n: int| length_end(line@, n) ==> end <= n,
        decreases line.len() - end,
    {
        if line[end] < 48 || line[end] > 57 {
            return Err("E_VERUS_SOURCES: malformed checksum length".to_string());
        }
        end += 1;
    }
    if end == 92 || end == line.len() || end == line.len() - 1
        || (end > 93 && line[92] == 48) {
        return Err("E_VERUS_SOURCES: malformed checksum length or path".to_string());
    }
    proof {
        assert(length_end(line@, end as int));
        assert forall|n: int| length_end(line@, n) implies n == end by {
            unique_length_end(line@, n, end as int);
        }
    }
    let mut i = end + 1;
    while i < line.len()
        invariant end + 1 <= i <= line.len(), end < line.len() - 1,
            length_end(line@, end as int),
            forall|n: int| length_end(line@, n) ==> n == end,
            forall|j: int| end < j < i ==>
                #[trigger] line[j] != 0 && line[j] != 10 && line[j] != 13,
        decreases line.len() - i,
    {
        if line[i] == 0 || line[i] == 10 || line[i] == 13 {
            return Err("E_VERUS_SOURCES: malformed checksum path".to_string());
        }
        i += 1;
    }
    let sha256 = copy_bytes(&line[18..82]);
    let length = copy_bytes(&line[92..end]);
    let path = copy_bytes(&line[end + 1..]);
    proof {
        assert(source_path(line@.subrange(end + 1, line.len() as int)));
        assert(line@ == line@.subrange(0, 18) + line@.subrange(18, 82)
            + line@.subrange(82, 92) + line@.subrange(92, end as int)
            + seq![32u8] + line@.subrange(end + 1, line.len() as int)) by {
            assert(line[end as int] == 32);
        }
    }
    Ok(SourceChecksum { path, sha256, length })
}

}
