//! Unix runner-owned launch tree. Anonymous inherited descriptors carry ownership,
//! never argv/environment policy. Each launcher reaps its own children on cancellation.
use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    ptr::NonNull,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
const TREE_FD: i32 = 198;
const CONTEXT_FD: i32 = 199;
const MAGIC: u64 = 0x4348524f4e4f5031;
const CAPACITY: usize = 4096;
#[repr(C)]
struct Slot {
    state: AtomicU8,
    cancelled: AtomicBool,
    pid: AtomicI32,
    parent: AtomicUsize,
}
#[repr(C)]
struct Shared {
    magic: u64,
    slots: [Slot; CAPACITY],
}
struct Tree {
    file: File,
    memory: NonNull<Shared>,
}
// MAP_SHARED is synchronized exclusively by the atomic slot protocol.
unsafe impl Send for Tree {}
unsafe impl Sync for Tree {}
impl Drop for Tree {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.memory.as_ptr().cast(), std::mem::size_of::<Shared>());
        }
    }
}
impl Tree {
    fn slots(&self) -> &[Slot; CAPACITY] {
        unsafe { &self.memory.as_ref().slots }
    }
    fn map(file: File) -> Result<Arc<Self>, String> {
        if file.metadata().map_err(|e| e.to_string())?.len() != std::mem::size_of::<Shared>() as u64
        {
            return Err("process ownership descriptor size mismatch".into());
        }
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                std::mem::size_of::<Shared>(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if p == libc::MAP_FAILED {
            return Err(format!(
                "process ownership map: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Arc::new(Self {
            file,
            memory: NonNull::new(p.cast()).unwrap(),
        }))
    }
    fn create() -> Result<Arc<Self>, String> {
        let file = high_descriptor(tempfile::tempfile().map_err(|e| e.to_string())?)?;
        file.set_len(std::mem::size_of::<Shared>() as u64)
            .map_err(|e| e.to_string())?;
        let tree = Self::map(file)?;
        unsafe {
            tree.memory.as_ptr().write(Shared {
                magic: MAGIC,
                slots: std::array::from_fn(|_| Slot {
                    state: AtomicU8::new(0),
                    cancelled: AtomicBool::new(false),
                    pid: AtomicI32::new(0),
                    parent: AtomicUsize::new(CAPACITY),
                }),
            });
        }
        Ok(tree)
    }
    fn cancelled(&self, mut index: usize) -> bool {
        for _ in 0..CAPACITY {
            if index == CAPACITY {
                return false;
            }
            let slot = &self.slots()[index];
            if slot.cancelled.load(Ordering::Acquire) {
                return true;
            }
            index = slot.parent.load(Ordering::Acquire);
        }
        true
    }
    fn descendant(&self, mut index: usize, ancestor: usize) -> bool {
        for _ in 0..CAPACITY {
            if index == CAPACITY {
                return false;
            }
            index = self.slots()[index].parent.load(Ordering::Acquire);
            if index == ancestor {
                return true;
            }
        }
        false
    }
}
fn high_descriptor(file: File) -> Result<File, String> {
    let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 202) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn inherited() -> Result<Option<(Arc<Tree>, usize)>, String> {
    static INHERITED: OnceLock<Result<Option<(Arc<Tree>, usize)>, String>> = OnceLock::new();
    INHERITED
        .get_or_init(|| {
            let mut magic = 0u64;
            let n = unsafe { libc::pread(TREE_FD, (&mut magic as *mut u64).cast(), 8, 0) };
            if n < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF) {
                return Ok(None);
            }
            if n != 8 || magic != MAGIC {
                return Err("process ownership descriptor collision".into());
            }
            let mut index = CAPACITY as u64;
            if unsafe { libc::pread(CONTEXT_FD, (&mut index as *mut u64).cast(), 8, 0) } != 8
                || index >= CAPACITY as u64
            {
                return Err("process ownership context missing".into());
            }
            let fd = unsafe { libc::fcntl(TREE_FD, libc::F_DUPFD_CLOEXEC, 202) };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let tree = Tree::map(unsafe { File::from_raw_fd(fd) })?;
            Ok(Some((tree, index as usize)))
        })
        .clone()
}
pub(crate) struct Launch {
    tree: Arc<Tree>,
    index: usize,
    context: File,
    acknowledged: AtomicBool,
}
impl Drop for Launch {
    fn drop(&mut self) {
        let slot = &self.tree.slots()[self.index];
        slot.pid.store(0, Ordering::Release);
        // A crashed nested owner must not let its ancestry slot be reused by
        // an unrelated launch while retained descendants still reference it.
        if self.acknowledged.load(Ordering::Acquire) {
            slot.state.store(0, Ordering::Release);
        }
    }
}
impl Launch {
    pub(crate) fn prepare() -> Result<Self, String> {
        let (tree, parent) = match inherited()? {
            Some(v) => v,
            None => (Tree::create()?, CAPACITY),
        };
        let index = tree
            .slots()
            .iter()
            .position(|s| {
                s.state
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            })
            .ok_or("process ownership capacity exhausted")?;
        let slot = &tree.slots()[index];
        slot.pid.store(0, Ordering::Release);
        slot.cancelled.store(false, Ordering::Release);
        slot.parent.store(parent, Ordering::Release);
        slot.state.store(2, Ordering::Release);
        let context = match tempfile::tempfile()
            .map_err(|e| e.to_string())
            .and_then(high_descriptor)
        {
            Ok(f) => f,
            Err(e) => {
                slot.state.store(0, Ordering::Release);
                return Err(e.to_string());
            }
        };
        let launch = Self {
            tree,
            index,
            context,
            acknowledged: AtomicBool::new(true),
        };
        use std::os::unix::fs::FileExt;
        launch
            .context
            .write_all_at(&(index as u64).to_ne_bytes(), 0)
            .map_err(|e| e.to_string())?;
        if launch.cancelled() {
            return Err("process cancelled by enclosing owner before launch".into());
        }
        Ok(launch)
    }
    pub(crate) fn configure(&self, command: &mut std::process::Command) {
        use std::os::unix::process::CommandExt;
        let tree_fd = self.tree.file.as_raw_fd();
        let context_fd = self.context.as_raw_fd();
        let memory = self.tree.memory.as_ptr() as usize;
        let index = self.index;
        unsafe {
            command.pre_exec(move || {
                let shared = &*(memory as *const Shared);
                shared.slots[index]
                    .pid
                    .store(libc::getpid(), Ordering::Release);
                if libc::dup2(tree_fd, TREE_FD) < 0 || libc::dup2(context_fd, CONTEXT_FD) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(TREE_FD, libc::F_SETFD, 0) < 0
                    || libc::fcntl(CONTEXT_FD, libc::F_SETFD, 0) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                let mut at = index;
                for _ in 0..CAPACITY {
                    if at == CAPACITY {
                        return Ok(());
                    }
                    if shared.slots[at].cancelled.load(Ordering::Acquire) {
                        return Err(std::io::Error::other(
                            "process cancelled by enclosing owner before exec",
                        ));
                    }
                    at = shared.slots[at].parent.load(Ordering::Acquire);
                }
                Err(std::io::Error::other("invalid process ownership ancestry"))
            });
        }
    }
    pub(crate) fn cancelled(&self) -> bool {
        self.tree.cancelled(self.index)
    }
    /// Cancellation is scoped to this launch. Allow nested engines to kill and
    /// reap their children before terminating the enclosing group. If an owner
    /// crashed, terminate only its explicitly recorded groups and report failure.
    pub(crate) fn drain(&self) -> bool {
        self.tree.slots()[self.index]
            .cancelled
            .store(true, Ordering::Release);
        let start = Instant::now();
        loop {
            let descendants: Vec<_> = self
                .tree
                .slots()
                .iter()
                .enumerate()
                .filter(|(i, s)| {
                    *i != self.index
                        && s.state.load(Ordering::Acquire) == 2
                        && self.tree.descendant(*i, self.index)
                })
                .map(|(_, s)| s.pid.load(Ordering::Acquire))
                .collect();
            let initializing = self
                .tree
                .slots()
                .iter()
                .any(|s| s.state.load(Ordering::Acquire) == 1);
            if descendants.is_empty() && !initializing {
                return true;
            }
            if start.elapsed() >= Duration::from_secs(1) {
                self.acknowledged.store(false, Ordering::Release);
                for pid in descendants {
                    if pid > 0 {
                        unsafe {
                            libc::kill(-pid, libc::SIGKILL);
                        }
                    }
                }
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
