//! Unix runner-owned launch tree. Anonymous inherited descriptors carry ownership,
//! never argv/environment policy. Each launcher reaps its own children on cancellation.
use std::{
    collections::BTreeMap,
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    ptr::NonNull,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
const TREE_FD: i32 = 198;
const CONTEXT_FD: i32 = 199;
const MAGIC: u64 = 0x4348524f4e4f5033;
const OBSERVER_FD: i32 = 200;
// Private copies must not alias the fixed inherited transport descriptors.
pub(crate) const PRIVATE_FD_MIN: i32 = 202;
const CAPACITY: usize = 4096;

#[repr(i32)]
enum LaunchStage {
    HandoffSend = 1,
    HandoffPoll,
    HandoffRead,
    Registration,
    Descriptors,
    DescriptorFlags,
    Ancestry,
    HandoffClock,
}
#[repr(i32)]
enum LaunchReason {
    Os = 1,
    Unavailable,
    IdentityRetired,
    MissingErrno,
    Cancelled,
    InvalidAncestry,
}
// Stack-only child errors. The private diagnostic follows the unchanged two-u64
// launch identity. No formatting, allocation or allocator-backed error may occur
// between fork and exec, even when publishing this record fails.
struct LaunchFailure {
    stage: LaunchStage,
    reason: LaunchReason,
    errno: i32,
}

// The existing private handoff socket also carries wake hints after the child
// has consumed its registration acknowledgement. Hints never establish exit.
#[derive(Clone)]
pub(crate) struct Wake(Arc<File>);
impl Wake {
    pub(crate) fn notify(&self) {
        notify(self.0.as_raw_fd());
    }
}
fn notify(fd: i32) {
    let hint = 1u8;
    unsafe {
        libc::send(
            fd,
            (&hint as *const u8).cast(),
            1,
            libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
        );
    }
}
impl LaunchFailure {
    fn os(stage: LaunchStage, errno: i32) -> Self {
        Self {
            stage,
            reason: LaunchReason::Os,
            errno,
        }
    }
    fn last_os(stage: LaunchStage) -> Self {
        Self::os(
            stage,
            std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL),
        )
    }
    fn owner(stage: LaunchStage, reason: LaunchReason) -> Self {
        Self {
            stage,
            reason,
            errno: 0,
        }
    }
    fn publish(self, fd: i32) -> std::io::Error {
        let record = [self.stage as i32, self.reason as i32, self.errno];
        let bytes = unsafe {
            std::slice::from_raw_parts(record.as_ptr().cast::<u8>(), std::mem::size_of_val(&record))
        };
        let mut written = 0;
        // Bound retries, including EINTR. Partial/unavailable diagnostics never
        // change the original errno transported by Command's exec error pipe.
        for _ in 0..bytes.len() {
            let n = unsafe {
                libc::pwrite(
                    fd,
                    bytes[written..].as_ptr().cast(),
                    bytes.len() - written,
                    16 + written as libc::off_t,
                )
            };
            if n > 0 {
                written += n as usize;
                if written == bytes.len() {
                    break;
                }
            } else if n == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR)
            {
                break;
            }
        }
        std::io::Error::from_raw_os_error(if self.errno == 0 {
            libc::EINVAL
        } else {
            self.errno
        })
    }
}

fn handoff_clock() -> Result<u128, LaunchFailure> {
    let mut now: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) } < 0 {
        return Err(LaunchFailure::last_os(LaunchStage::HandoffClock));
    }
    Ok((now.tv_sec as u128)
        .saturating_mul(1_000_000_000)
        .saturating_add(now.tv_nsec as u128))
}
fn monotonic_ns() -> Option<u64> {
    // Match std::time::Instant, including Apple's suspend behavior. The
    // process-local representation of Instant itself cannot cross this mapping.
    #[cfg(target_vendor = "apple")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_vendor = "apple"))]
    let clock = libc::CLOCK_MONOTONIC;
    let mut now: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(clock, &mut now) } < 0 {
        return None;
    }
    let seconds = u64::try_from(now.tv_sec).ok()?;
    let nanoseconds = u64::try_from(now.tv_nsec).ok()?;
    seconds.checked_mul(1_000_000_000)?.checked_add(nanoseconds)
}
// The shared observer can run in a different process. Use the same monotonic
// clock domain; an unavailable or unrepresentable reading supplies no evidence.
pub(crate) fn terminal_deadline(timeout: Duration) -> Option<u64> {
    monotonic_ns()?.checked_add(timeout.as_nanos().try_into().ok()?)
}
#[repr(C)]
struct Slot {
    state: AtomicU8,
    cancelled: AtomicBool,
    pid: AtomicI32,
    parent: AtomicUsize,
    generation: AtomicUsize,
    terminal: AtomicBool,
    terminal_observed_ns: AtomicU64,
    launcher_terminal: AtomicBool,
}
#[repr(C)]
struct Shared {
    magic: u64,
    next_generation: AtomicUsize,
    slots: [Slot; CAPACITY],
}
struct Tree {
    file: File,
    memory: NonNull<Shared>,
    registration: File,
    observer: Option<ExitObserver>,
}
// MAP_SHARED is synchronized exclusively by the atomic slot protocol.
unsafe impl Send for Tree {}
unsafe impl Sync for Tree {}
impl Drop for Tree {
    fn drop(&mut self) {
        if let Some(mut observer) = self.observer.take() {
            observer.join();
        }
        unsafe {
            libc::munmap(self.memory.as_ptr().cast(), std::mem::size_of::<Shared>());
        }
    }
}
impl Tree {
    fn memory(&self) -> &Shared {
        unsafe { self.memory.as_ref() }
    }
    fn slots(&self) -> &[Slot; CAPACITY] {
        unsafe { &self.memory.as_ref().slots }
    }
    fn map(file: File, registration: File) -> Result<Arc<Self>, String> {
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
            registration,
            observer: None,
        }))
    }
    fn create() -> Result<Arc<Self>, String> {
        let file = high_descriptor(tempfile::tempfile().map_err(|e| e.to_string())?)?;
        file.set_len(std::mem::size_of::<Shared>() as u64)
            .map_err(|e| e.to_string())?;
        let (sender, receiver) =
            std::os::unix::net::UnixDatagram::pair().map_err(|e| e.to_string())?;
        let mut tree = Self::map(
            file,
            high_descriptor(File::from(std::os::fd::OwnedFd::from(sender)))?,
        )?;
        unsafe {
            tree.memory.as_ptr().write(Shared {
                magic: MAGIC,
                next_generation: AtomicUsize::new(1),
                slots: std::array::from_fn(|_| Slot {
                    state: AtomicU8::new(0),
                    cancelled: AtomicBool::new(false),
                    pid: AtomicI32::new(0),
                    parent: AtomicUsize::new(CAPACITY),
                    generation: AtomicUsize::new(0),
                    terminal: AtomicBool::new(false),
                    terminal_observed_ns: AtomicU64::new(0),
                    launcher_terminal: AtomicBool::new(false),
                }),
            });
        }
        let observer = ExitObserver::start(
            File::from(std::os::fd::OwnedFd::from(receiver)),
            tree.memory.as_ptr() as usize,
        )?;
        Arc::get_mut(&mut tree).unwrap().observer = Some(observer);
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
    fn descendant(&self, index: usize, ancestor: usize) -> bool {
        descendant(self.slots(), index, ancestor)
    }
}
fn descendant(slots: &[Slot; CAPACITY], mut index: usize, ancestor: usize) -> bool {
    for _ in 0..CAPACITY {
        if index == CAPACITY {
            return false;
        }
        index = slots[index].parent.load(Ordering::Acquire);
        if index == ancestor {
            return true;
        }
    }
    false
}

fn high_descriptor(file: File) -> Result<File, String> {
    let fd = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, PRIVATE_FD_MIN) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn inherited() -> Result<Option<(Arc<Tree>, usize, usize)>, String> {
    static INHERITED: OnceLock<Result<Option<(Arc<Tree>, usize, usize)>, String>> = OnceLock::new();
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
            let mut context = [CAPACITY as u64, 0];
            if unsafe { libc::pread(CONTEXT_FD, context.as_mut_ptr().cast(), 16, 0) } != 16
                || context[0] >= CAPACITY as u64
            {
                return Err("process ownership context missing".into());
            }
            let fd = unsafe { libc::fcntl(TREE_FD, libc::F_DUPFD_CLOEXEC, PRIVATE_FD_MIN) };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let observer_fd =
                unsafe { libc::fcntl(OBSERVER_FD, libc::F_DUPFD_CLOEXEC, PRIVATE_FD_MIN) };
            if observer_fd < 0 {
                return Err("process ownership exit observer missing".into());
            }
            let tree = Tree::map(unsafe { File::from_raw_fd(fd) }, unsafe {
                File::from_raw_fd(observer_fd)
            })?;
            Ok(Some((tree, context[0] as usize, context[1] as usize)))
        })
        .clone()
}
pub(crate) struct Launch {
    tree: Arc<Tree>,
    index: usize,
    context: File,
    acknowledged: AtomicBool,
    acknowledgement: File,
    acknowledgement_sender: Arc<File>,
}
impl Drop for Launch {
    fn drop(&mut self) {
        let slot = &self.tree.slots()[self.index];
        // A crashed nested owner must not let its ancestry slot be reused by
        // an unrelated launch while retained descendants still reference it.
        // Only the observer releases acknowledged slots for reuse, after its
        // current batch of identity events has finished writing their markers.
        if self.acknowledged.load(Ordering::Acquire) {
            slot.state.store(3, Ordering::Release);
        }
    }
}
impl Launch {
    pub(crate) fn prepare() -> Result<Self, String> {
        let (tree, parent, parent_generation) = match inherited()? {
            Some(v) => v,
            None => (Tree::create()?, CAPACITY, 0),
        };
        if parent != CAPACITY
            && (tree.slots()[parent].state.load(Ordering::Acquire) != 2
                || tree.slots()[parent].generation.load(Ordering::Acquire) != parent_generation)
        {
            return Err("process ownership inherited launch identity retired".into());
        }
        // Acquire fallible resources before publishing a reservation. A failed
        // descriptor allocation has no launched child and must not strand a slot.
        let context = high_descriptor(tempfile::tempfile().map_err(|e| e.to_string())?)?;
        let (acknowledgement, acknowledgement_sender) =
            std::os::unix::net::UnixStream::pair().map_err(|e| e.to_string())?;
        let acknowledgement =
            high_descriptor(File::from(std::os::fd::OwnedFd::from(acknowledgement)))?;
        let acknowledgement_sender = high_descriptor(File::from(std::os::fd::OwnedFd::from(
            acknowledgement_sender,
        )))?;
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
        slot.generation.store(
            tree.memory().next_generation.fetch_add(1, Ordering::AcqRel),
            Ordering::Release,
        );
        slot.terminal.store(false, Ordering::Release);
        slot.terminal_observed_ns.store(0, Ordering::Release);
        slot.launcher_terminal.store(false, Ordering::Release);
        slot.cancelled.store(false, Ordering::Release);
        slot.parent.store(parent, Ordering::Release);
        slot.state.store(2, Ordering::Release);
        let launch = Self {
            tree,
            index,
            context,
            acknowledged: AtomicBool::new(true),
            acknowledgement,
            acknowledgement_sender: Arc::new(acknowledgement_sender),
        };
        use std::os::unix::fs::FileExt;
        launch
            .context
            .write_all_at(&(index as u64).to_ne_bytes(), 0)
            .map_err(|e| e.to_string())?;
        launch
            .context
            .write_all_at(
                &(launch.tree.slots()[index]
                    .generation
                    .load(Ordering::Acquire) as u64)
                    .to_ne_bytes(),
                8,
            )
            .map_err(|e| e.to_string())?;
        if launch.cancelled()
            || (parent != CAPACITY
                && launch.tree.slots()[parent]
                    .generation
                    .load(Ordering::Acquire)
                    != parent_generation)
        {
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
        let observer_fd = self.tree.registration.as_raw_fd();
        let acknowledgement_fd = self.acknowledgement.as_raw_fd();
        let sender_fd = self.acknowledgement_sender.as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                let result = (|| {
                    let shared = &*(memory as *const Shared);
                    shared.slots[index]
                        .pid
                        .store(libc::getpid(), Ordering::Release);
                    register_exit(
                        observer_fd,
                        acknowledgement_fd,
                        sender_fd,
                        index,
                        shared.slots[index].generation.load(Ordering::Acquire),
                    )?;
                    if libc::dup2(tree_fd, TREE_FD) < 0
                        || libc::dup2(context_fd, CONTEXT_FD) < 0
                        || libc::dup2(observer_fd, OBSERVER_FD) < 0
                    {
                        return Err(LaunchFailure::last_os(LaunchStage::Descriptors));
                    }
                    if libc::fcntl(TREE_FD, libc::F_SETFD, 0) < 0
                        || libc::fcntl(CONTEXT_FD, libc::F_SETFD, 0) < 0
                        || libc::fcntl(OBSERVER_FD, libc::F_SETFD, 0) < 0
                    {
                        return Err(LaunchFailure::last_os(LaunchStage::DescriptorFlags));
                    }
                    let mut at = index;
                    for _ in 0..CAPACITY {
                        if at == CAPACITY {
                            return Ok(());
                        }
                        if shared.slots[at].cancelled.load(Ordering::Acquire) {
                            return Err(LaunchFailure::owner(
                                LaunchStage::Ancestry,
                                LaunchReason::Cancelled,
                            ));
                        }
                        at = shared.slots[at].parent.load(Ordering::Acquire);
                    }
                    Err(LaunchFailure::owner(
                        LaunchStage::Ancestry,
                        LaunchReason::InvalidAncestry,
                    ))
                })();
                result.map_err(|failure| failure.publish(context_fd))
            });
        }
    }
    pub(crate) fn spawn_error(&self, error: std::io::Error) -> String {
        use std::os::unix::fs::FileExt;
        let mut bytes = [0u8; 12];
        if self.context.read_exact_at(&mut bytes, 16).is_err() {
            return error.to_string();
        }
        let [stage, reason, errno] = std::array::from_fn(|i| {
            i32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        // Render only in the parent, after spawn has returned and reaped a
        // failed pre-exec child. Incomplete or unknown records are diagnostic
        // absence, never evidence of a process exit or successful registration.
        let detail = match (stage, reason) {
            (1, 1) => "process ownership exit handoff send",
            (2, 1) => "process ownership live exit handoff poll",
            (3, 1) => "process ownership live exit handoff read",
            (2, 2) => "process ownership live exit handoff failed (poll timeout)",
            (3, 2) => "process ownership live exit handoff failed (short read)",
            (4, 1) => "process ownership live exit registration",
            (4, 3) => "process ownership live exit handoff identity retired",
            (4, 4) => "process ownership live exit registration failed without OS errno",
            (5, 1) => "process ownership descriptor duplication",
            (6, 1) => "process ownership descriptor flags",
            (7, 5) => "process cancelled by enclosing owner before exec",
            (7, 6) => "invalid process ownership ancestry",
            (8, 1) => "process ownership live exit handoff clock",
            _ => return error.to_string(),
        };
        if errno == 0 {
            format!("{error}; {detail}")
        } else {
            format!(
                "{error}; {detail}: {}",
                std::io::Error::from_raw_os_error(errno)
            )
        }
    }
    pub(crate) fn cancelled(&self) -> bool {
        self.tree.cancelled(self.index)
    }
    pub(crate) fn observed_exit_before(&self, deadline: Option<u64>) -> bool {
        let observed = self.tree.slots()[self.index]
            .terminal_observed_ns
            .load(Ordering::Acquire);
        deadline.is_some_and(|deadline| observed != 0 && observed < deadline)
    }
    pub(crate) fn wake(&self) -> Wake {
        Wake(self.acknowledgement_sender.clone())
    }
    pub(crate) fn wait_for_wake(&self, interval: Duration) {
        let mut pending = libc::pollfd {
            fd: self.acknowledgement.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout = interval.as_millis().min(i32::MAX as u128) as i32;
        if unsafe { libc::poll(&mut pending, 1, timeout) } > 0 {
            // Consume only an advisory notification. The process monitor must
            // still probe Child::try_wait and enforce cancellation/output bounds.
            let mut hints = [0u8; 32];
            unsafe {
                libc::recv(
                    pending.fd,
                    hints.as_mut_ptr().cast(),
                    hints.len(),
                    libc::MSG_DONTWAIT,
                );
            }
        }
    }
    /// Cancellation is scoped to this launch. Allow nested engines to kill and
    /// reap their children before terminating the enclosing group. If an owner
    /// crashed, terminate only its explicitly recorded groups and report failure.
    pub(crate) fn drain(&self) -> bool {
        self.tree.slots()[self.index]
            .cancelled
            .store(true, Ordering::Release);
        let start = Instant::now();
        let mut abandoned = Vec::new();
        loop {
            // Keep the existing one-second cleanup allowance. If an interrupted
            // launcher still has a live child halfway through it, terminate that
            // explicit group, then require its kernel exit acknowledgement in
            // the remaining allowance. That abandoned-live-child outcome stays
            // a failure even when cleanup establishes terminal completion.
            if start.elapsed() >= Duration::from_millis(500) {
                for (i, slot) in self.tree.slots().iter().enumerate() {
                    if i != self.index
                        && slot.state.load(Ordering::Acquire) == 2
                        && self.tree.descendant(i, self.index)
                        && slot.launcher_terminal.load(Ordering::Acquire)
                        && !slot.terminal.load(Ordering::Acquire)
                        && !abandoned.contains(&i)
                    {
                        abandoned.push(i);
                        let pid = slot.pid.load(Ordering::Acquire);
                        if pid > 0 {
                            unsafe {
                                libc::kill(-pid, libc::SIGKILL);
                            }
                        }
                    }
                }
            }
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
                return abandoned.is_empty();
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

// The first engine owns one joined observer for its finite launch tree. Children
// hand off a live identity before exec; the observer stays with the surviving
// root even if an intermediate launcher is killed. No process discovery occurs.
struct ExitObserver {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ExitObserver {
    fn join(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
    fn start(receiver: File, memory: usize) -> Result<Self, String> {
        let flags = unsafe { libc::fcntl(receiver.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe {
                libc::fcntl(
                    receiver.as_raw_fd(),
                    libc::F_SETFL,
                    flags | libc::O_NONBLOCK,
                )
            } < 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut identities = ExitIdentities::new()?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = std::thread::Builder::new()
            .name("chrono-owned-exits".into())
            .spawn(move || {
                let shared = unsafe { &*(memory as *const Shared) };
                let mut wake = BTreeMap::new();
                let diagnostic = std::env::var_os("CHRONO_FAILED_HANDOFF").is_some();
                let mut emitted = false;
                while !stopping.load(Ordering::Acquire) {
                    // Receive only explicit launch handoffs. The reply descriptor is
                    // specific to this child, so sibling registrations cannot steal it.
                    loop {
                        let mut data = [0usize; 4];
                        let mut control = [0usize; 8];
                        let mut iov = libc::iovec {
                            iov_base: data.as_mut_ptr().cast(),
                            iov_len: std::mem::size_of_val(&data),
                        };
                        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
                        msg.msg_iov = &mut iov;
                        msg.msg_iovlen = 1;
                        msg.msg_control = control.as_mut_ptr().cast();
                        msg.msg_controllen = std::mem::size_of_val(&control) as _;
                        let n = unsafe { libc::recvmsg(receiver.as_raw_fd(), &mut msg, 0) };
                        let received_error = std::io::Error::last_os_error();
                        if diagnostic && !emitted && (n >= 0 || received_error.raw_os_error() != Some(libc::EAGAIN)) {
                            emitted = true;
                            let occupied: Vec<_> = (190..220).filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } >= 0).collect();
                            eprintln!("handoff diagnostic recv={n} errno={:?} flags={} control={} data={data:?} occupied={occupied:?}", received_error.raw_os_error(), msg.msg_flags, msg.msg_controllen);
                        }
                        if n < 0 {
                            break;
                        }
                        let header = unsafe { libc::CMSG_FIRSTHDR(&msg) };
                        if n as usize != std::mem::size_of_val(&data) || header.is_null() {
                            continue;
                        }
                        let reply =
                            unsafe { File::from_raw_fd(*(libc::CMSG_DATA(header).cast::<i32>())) };
                        let [index, generation, pid, launcher] = data;
                        // The observer retains this endpoint until exit. It is
                        // private notification state, not a child capability.
                        let ack: i32 = if unsafe {
                            libc::fcntl(reply.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC)
                        } < 0
                        {
                            std::io::Error::last_os_error().raw_os_error().unwrap_or(-2)
                        } else if index < CAPACITY
                            && shared.slots[index].generation.load(Ordering::Acquire) == generation
                        {
                            identities
                                .register(
                                    pid as i32,
                                    launcher as i32,
                                    generation * CAPACITY * 2 + index * 2,
                                )
                                .err()
                                .map_or(0, |e| e.raw_os_error().unwrap_or(-2))
                        } else {
                            -1
                        };
                        unsafe {
                            libc::send(
                                reply.as_raw_fd(),
                                (&ack as *const i32).cast(),
                                std::mem::size_of_val(&ack),
                                libc::MSG_NOSIGNAL,
                            );
                        }
                        if ack == 0 {
                            wake.insert(generation * CAPACITY * 2 + index * 2, reply);
                        }
                    }
                    identities.retain(|cookie| {
                        let slot = &shared.slots[(cookie / 2) % CAPACITY];
                        slot.state.load(Ordering::Acquire) != 0
                            && slot.generation.load(Ordering::Acquire) == cookie / (CAPACITY * 2)
                    });
                    wake.retain(|cookie, _| {
                        let slot = &shared.slots[(*cookie / 2) % CAPACITY];
                        slot.state.load(Ordering::Acquire) != 0
                            && slot.generation.load(Ordering::Acquire) == *cookie / (CAPACITY * 2)
                    });
                    identities.completed(|cookie| {
                        let index = (cookie / 2) % CAPACITY;
                        let generation = cookie / (CAPACITY * 2);
                        if shared.slots[index].generation.load(Ordering::Acquire) == generation {
                            if cookie % 2 == 0 {
                                if let Some(observed) = monotonic_ns() {
                                    shared.slots[index]
                                        .terminal_observed_ns
                                        .store(observed, Ordering::Release);
                                }
                                shared.slots[index].terminal.store(true, Ordering::Release);
                                if let Some(reply) = wake.remove(&cookie) {
                                    notify(reply.as_raw_fd());
                                }
                            } else {
                                shared.slots[index]
                                    .launcher_terminal
                                    .store(true, Ordering::Release);
                            }
                        }
                    });
                    // This thread is the sole slot recycler. Complete the event
                    // batch before freeing any slot: a delayed event cannot pass
                    // a generation check and then write into a reused slot.
                    for (index, slot) in shared.slots.iter().enumerate() {
                        if slot.state.load(Ordering::Acquire) == 3 {
                            slot.state.store(0, Ordering::Release);
                            continue;
                        }
                        if slot.state.load(Ordering::Acquire) == 2
                            && slot.terminal.load(Ordering::Acquire)
                            && slot.launcher_terminal.load(Ordering::Acquire)
                            && !shared.slots.iter().enumerate().any(|(child, child_slot)| {
                                child != index
                                    && child_slot.state.load(Ordering::Acquire) == 2
                                    && descendant(&shared.slots, child, index)
                            })
                        {
                            slot.state.store(0, Ordering::Release);
                        }
                    }
                    // Both new handoffs and already registered exits wake the
                    // observer. A quiet registration socket must not defer an
                    // available exit until the periodic maintenance interval.
                    identities.wait(receiver.as_raw_fd());
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}
fn register_exit(
    observer: i32,
    acknowledgement: i32,
    reply: i32,
    index: usize,
    generation: usize,
) -> Result<(), LaunchFailure> {
    let mut data = [
        index,
        generation,
        unsafe { libc::getpid() } as usize,
        unsafe { libc::getppid() } as usize,
    ];
    let mut control = [0usize; 8];
    let mut iov = libc::iovec {
        iov_base: data.as_mut_ptr().cast(),
        iov_len: std::mem::size_of_val(&data),
    };
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr().cast();
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as _) } as _;
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&msg);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as _) as _;
        *(libc::CMSG_DATA(header).cast::<i32>()) = reply;
        let start = handoff_clock()?;
        loop {
            if libc::sendmsg(observer, &msg, libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL) >= 0 {
                break;
            }
            let failure = LaunchFailure::last_os(LaunchStage::HandoffSend);
            if !matches!(failure.errno, libc::EAGAIN | libc::EINTR | libc::ENOBUFS)
                || handoff_clock()?.saturating_sub(start) >= 1_000_000_000
            {
                return Err(failure);
            }
            // ENOBUFS may succeed once kernel buffers become available, just as
            // finite anonymous transport can exert backpressure under ordinary
            // concurrent launches. No child has exec'd; retain one handoff and
            // the existing one-second bound instead of losing its registration.
            let pause = libc::timespec {
                tv_sec: 0,
                tv_nsec: 1_000_000,
            };
            libc::nanosleep(&pause, std::ptr::null_mut());
        }
        let mut poll = libc::pollfd {
            fd: acknowledgement,
            events: libc::POLLIN,
            revents: 0,
        };
        let mut ack = 0i32;
        let polled = libc::poll(
            &mut poll,
            1,
            (1000u128.saturating_sub(handoff_clock()?.saturating_sub(start) / 1_000_000)) as i32,
        );
        if polled < 0 {
            return Err(LaunchFailure::last_os(LaunchStage::HandoffPoll));
        }
        if polled != 1 {
            return Err(LaunchFailure::owner(
                LaunchStage::HandoffPoll,
                LaunchReason::Unavailable,
            ));
        }
        let received = libc::read(
            acknowledgement,
            (&mut ack as *mut i32).cast(),
            std::mem::size_of_val(&ack),
        );
        if received < 0 {
            return Err(LaunchFailure::last_os(LaunchStage::HandoffRead));
        }
        if received != std::mem::size_of_val(&ack) as isize {
            return Err(LaunchFailure::owner(
                LaunchStage::HandoffRead,
                LaunchReason::Unavailable,
            ));
        }
        if ack == -1 {
            return Err(LaunchFailure::owner(
                LaunchStage::Registration,
                LaunchReason::IdentityRetired,
            ));
        }
        if ack == -2 {
            return Err(LaunchFailure::owner(
                LaunchStage::Registration,
                LaunchReason::MissingErrno,
            ));
        }
        if ack != 0 {
            return Err(LaunchFailure::os(LaunchStage::Registration, ack));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
struct ExitIdentities(Vec<(File, usize)>);
#[cfg(target_os = "macos")]
impl ExitIdentities {
    fn new() -> Result<Self, String> {
        Ok(Self(Vec::new()))
    }
    fn register(&mut self, pid: i32, launcher: i32, cookie: usize) -> std::io::Result<()> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let queue = unsafe { File::from_raw_fd(fd) };
        unsafe {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
        let events = [(pid, cookie), (launcher, cookie + 1)].map(|(pid, cookie)| libc::kevent {
            ident: pid as _,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ONESHOT,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: cookie as *mut libc::c_void,
        });
        if unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                events.as_ptr(),
                2,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        } < 0
        {
            return Err(std::io::Error::last_os_error());
        }
        self.0.push((queue, cookie));
        Ok(())
    }
    fn retain(&mut self, mut active: impl FnMut(usize) -> bool) {
        self.0.retain(|(_, cookie)| active(*cookie));
    }
    fn completed(&mut self, mut accept: impl FnMut(usize)) {
        // One kernel readiness query for the declared handles; do not repeatedly
        // query every empty queue while unrelated fixture owners are running.
        let mut ready: Vec<_> = self
            .0
            .iter()
            .map(|(queue, _)| libc::pollfd {
                fd: queue.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        if unsafe { libc::poll(ready.as_mut_ptr(), ready.len() as _, 0) } <= 0 {
            return;
        }
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        for ((queue, _), ready) in self.0.iter().zip(ready) {
            if ready.revents & libc::POLLIN == 0 {
                continue;
            }
            let mut events: [libc::kevent; 2] = unsafe { std::mem::zeroed() };
            let n = unsafe {
                libc::kevent(
                    queue.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    2,
                    &timeout,
                )
            };
            for event in events.iter().take(n.max(0) as usize) {
                if event.flags & libc::EV_ERROR == 0 && event.fflags & libc::NOTE_EXIT != 0 {
                    accept(event.udata as usize);
                }
            }
        }
    }
}
#[cfg(target_os = "linux")]
struct ExitIdentities(Vec<(File, usize)>);
#[cfg(any(target_os = "linux", target_os = "macos"))]
impl ExitIdentities {
    fn wait(&self, registration: i32) {
        let mut pending: Vec<_> = std::iter::once(registration)
            .chain(self.0.iter().map(|(identity, _)| identity.as_raw_fd()))
            .map(|fd| libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        // Readiness is only a wake hint. completed() still obtains the actual
        // kernel event, checks its identity and records the observation time.
        // Keep the existing bound for stop, cancellation and slot recycling.
        unsafe { libc::poll(pending.as_mut_ptr(), pending.len() as _, 2) };
    }
}
#[cfg(target_os = "linux")]
impl ExitIdentities {
    fn new() -> Result<Self, String> {
        Ok(Self(Vec::new()))
    }
    fn register(&mut self, pid: i32, launcher: i32, cookie: usize) -> std::io::Result<()> {
        let open = |pid: libc::pid_t| {
            let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
            if fd < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(unsafe { File::from_raw_fd(fd) })
            }
        };
        let child = open(pid)?;
        let owner = open(launcher)?;
        self.0.extend([(child, cookie), (owner, cookie + 1)]);
        Ok(())
    }
    fn retain(&mut self, mut active: impl FnMut(usize) -> bool) {
        self.0.retain(|(_, cookie)| active(*cookie));
    }
    fn completed(&mut self, mut accept: impl FnMut(usize)) {
        let mut ready: Vec<_> = self
            .0
            .iter()
            .map(|(file, _)| libc::pollfd {
                fd: file.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        if unsafe { libc::poll(ready.as_mut_ptr(), ready.len() as _, 0) } <= 0 {
            return;
        }
        let mut index = 0;
        self.0.retain(|(_, cookie)| {
            let terminal = ready[index].revents & libc::POLLIN != 0;
            index += 1;
            if terminal {
                accept(*cookie);
            }
            !terminal
        });
    }
}
