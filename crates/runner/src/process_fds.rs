//! Explicit capability transfer through the existing process engine.
//! Owned copies stay CLOEXEC; inherited flags remain unchanged for native children.
use crate::ProcessFailure;
use std::{
    cell::RefCell,
    io,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
    process::Command,
};

pub const ENV: &str = "CHRONO_PROCESS_FDS";
thread_local! { static SCOPES: RefCell<Vec<(u64, Vec<OwnedFd>)>> = const { RefCell::new(Vec::new()) }; }
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn duplicate(fd: RawFd) -> Result<OwnedFd, ProcessFailure> {
    if fd < 3 {
        return Err("ownership descriptor must not alias stdio".into());
    }
    let duplicate = || {
        let copy = unsafe {
            libc::fcntl(
                fd,
                libc::F_DUPFD_CLOEXEC,
                crate::process_ownership::PRIVATE_FD_MIN,
            )
        };
        if copy < 0 {
            let error = io::Error::last_os_error();
            return Err(ProcessFailure::native(
                "E_PROCESS_OWNERSHIP_DUPLICATE",
                format!("ownership descriptor: {error}"),
                error,
            ));
        }
        Ok(unsafe { OwnedFd::from_raw_fd(copy) })
    };
    let copy = duplicate()?;
    // Bash 3.2 不检查占用就把脚本输入复制到 255；能力载体须避开它。
    // 第二次复制期间持有该槽位，也保留描述符耗尽时的原生错误。
    if copy.as_raw_fd() == 255 {
        return duplicate();
    }
    Ok(copy)
}
/// A thread-local, owned set of opaque capabilities forwarded by every engine call.
/// Dropping closes copies; it never unlocks a shared inherited open description.
pub struct Scope {
    id: u64,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Scope {
    pub fn new(fds: &[BorrowedFd<'_>]) -> Result<Self, String> {
        let copies = fds
            .iter()
            .map(|fd| duplicate(fd.as_raw_fd()))
            .collect::<Result<_, _>>()
            .map_err(|error: ProcessFailure| error.message)?;
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SCOPES.with(|s| s.borrow_mut().push((id, copies)));
        Ok(Self {
            id,
            _thread: std::marker::PhantomData,
        })
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        SCOPES.with(|s| s.borrow_mut().retain(|(id, _)| *id != self.id));
    }
}
fn inherited() -> Result<Vec<RawFd>, ProcessFailure> {
    let Some(value) = std::env::var_os(ENV) else {
        return Ok(vec![]);
    };
    let value = value.to_str().ok_or("non UTF-8 ownership descriptors")?;
    if value.is_empty() {
        return Err("empty ownership descriptor list".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    value
        .split(',')
        .map(|v| {
            let fd: RawFd = v.parse().map_err(|_| "invalid ownership descriptor")?;
            if fd < 3 || !seen.insert(fd) {
                return Err("invalid or duplicate ownership descriptor".into());
            }
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags < 0 {
                let error = io::Error::last_os_error();
                return Err(ProcessFailure::native(
                    "E_PROCESS_OWNERSHIP_INHERITED",
                    format!("inherited ownership descriptor {fd}: {error}"),
                    error,
                ));
            }
            Ok(fd)
        })
        .collect()
}
/// 查询原始继承能力是否存在；解析和 fd 有效性仍由同一个负责方维护。
pub fn inherited_available() -> Result<bool, String> {
    inherited()
        .map(|fds| !fds.is_empty())
        .map_err(|error| error.message)
}
pub(crate) struct Transfer {
    _copies: std::sync::Arc<Vec<OwnedFd>>,
    pub(crate) value: Option<String>,
}
impl Transfer {
    pub(crate) fn prepare(command: &mut Command) -> Result<Self, ProcessFailure> {
        let inherited = inherited()?;
        let mut copies = inherited
            .iter()
            .map(|fd| duplicate(*fd))
            .collect::<Result<Vec<_>, _>>()?;
        SCOPES.with(|s| -> Result<(), ProcessFailure> {
            for (_, fds) in s.borrow().iter() {
                for fd in fds {
                    copies.push(duplicate(fd.as_raw_fd())?);
                }
            }
            Ok(())
        })?;
        let copies = std::sync::Arc::new(copies);
        let mut value = None;
        if !copies.is_empty() {
            let fds: Vec<_> = copies.iter().map(AsRawFd::as_raw_fd).collect();
            let carrier = fds
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            command.env(ENV, &carrier);
            value = Some(carrier);
            use std::os::unix::process::CommandExt;
            let held = copies.clone();
            unsafe {
                command.pre_exec(move || {
                    // Command owns these copies too: explicit native adapters may
                    // drop their temporary Scope before spawning this command.
                    let _ = &held;
                    // Only this managed child retires the superseded carriers.
                    // The parent and its native children keep their original flags.
                    for fd in &inherited {
                        let flags = libc::fcntl(*fd, libc::F_GETFD);
                        if flags < 0
                            || libc::fcntl(*fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) < 0
                        {
                            return Err(io::Error::last_os_error());
                        }
                    }
                    for fd in &fds {
                        let flags = libc::fcntl(*fd, libc::F_GETFD);
                        if flags < 0
                            || libc::fcntl(*fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0
                        {
                            return Err(io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
        } else {
            command.env_remove(ENV);
        }
        Ok(Self {
            _copies: copies,
            value,
        })
    }
}

/// Forward the same opaque capabilities through a declared native Command route.
/// The command owns CLOEXEC copies until spawn/drop; only its child inherits them.
pub fn forward_command(command: &mut Command, fds: &[BorrowedFd<'_>]) -> Result<(), String> {
    if command.get_envs().any(|(key, _)| key == ENV) {
        return Err(
            "native command already has an ownership binding; configure it before binding once"
                .into(),
        );
    }
    let _scope = Scope::new(fds)?;
    Transfer::prepare(command).map_err(|failure| failure.to_string())?;
    Ok(())
}
