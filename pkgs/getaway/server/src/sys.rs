#![allow(unsafe_code)]

use rustix::{
    event::{EventfdFlags, PollFd, PollFlags, eventfd, poll},
    fs::{Mode, OFlags, open},
    time::Timespec,
};
use std::{
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    rc::Rc,
    sync::atomic::{AtomicI32, Ordering::SeqCst},
};

static TERMINATE: AtomicI32 = AtomicI32::new(-1);

pub fn termination() -> io::Result<OwnedFd> {
    let mut fds = [0; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
        return Err(io::Error::last_os_error());
    }
    TERMINATE.store(fds[1], SeqCst);
    for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP, libc::SIGCHLD] {
        unsafe { libc::signal(signal, on_signal as *const () as libc::sighandler_t) };
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fds[0]) })
}

extern "C" fn on_signal(signal: libc::c_int) {
    unsafe { libc::write(TERMINATE.load(SeqCst), [signal as u8].as_ptr().cast(), 1) };
}

pub fn signals(fd: &OwnedFd) -> (bool, bool) {
    let (mut terminate, mut child, mut buf) = (false, false, [0u8; 64]);
    while let Ok(n @ 1..) = rustix::io::read(fd, &mut buf) {
        child |= buf[..n].contains(&(libc::SIGCHLD as u8));
        terminate |= buf[..n].iter().any(|&s| s != libc::SIGCHLD as u8);
    }
    (terminate, child)
}

const DMA_BUF_IOCTL_EXPORT_SYNC_FILE: libc::c_ulong = 0xc008_6202;
const DMA_BUF_SYNC_READ: u32 = 1;

const fn drm_iowr(nr: u64, size: u64) -> libc::c_ulong {
    (3 << 30) | (size << 16) | (0x64 << 8) | nr
}

const SYNCOBJ_DESTROY: libc::c_ulong = drm_iowr(0xc0, 8);
const SYNCOBJ_FD_TO_HANDLE: libc::c_ulong = drm_iowr(0xc2, 16);
const SYNCOBJ_TIMELINE_SIGNAL: libc::c_ulong = drm_iowr(0xcd, 24);
const SYNCOBJ_EVENTFD: libc::c_ulong = drm_iowr(0xcf, 24);

#[repr(C)]
struct SyncFile {
    flags: u32,
    fd: i32,
}

#[repr(C)]
struct Handle {
    handle: u32,
    flags: u32,
    fd: i32,
    pad: u32,
}

#[repr(C)]
struct TimelineArray {
    handles: u64,
    points: u64,
    count: u32,
    flags: u32,
}

#[repr(C)]
struct Eventfd {
    handle: u32,
    flags: u32,
    point: u64,
    fd: i32,
    pad: u32,
}

fn ioctl<T>(fd: BorrowedFd, request: libc::c_ulong, arg: &mut T) -> io::Result<()> {
    match unsafe { libc::ioctl(fd.as_raw_fd(), request, arg as *mut T) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

pub fn pending_writes(dmabuf: BorrowedFd) -> Option<OwnedFd> {
    let mut export = SyncFile { flags: DMA_BUF_SYNC_READ, fd: -1 };
    ioctl(dmabuf, DMA_BUF_IOCTL_EXPORT_SYNC_FILE, &mut export).ok()?;
    let fence = unsafe { OwnedFd::from_raw_fd(export.fd) };
    (!signaled(&fence)).then_some(fence)
}

pub fn signaled(fd: &OwnedFd) -> bool {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    poll(&mut fds, Some(&Timespec { tv_sec: 0, tv_nsec: 0 })).is_ok_and(|n| n > 0)
}

pub struct Drm(OwnedFd);

impl Drm {
    pub fn open(render_node: u64) -> io::Result<Rc<Self>> {
        let path = format!("/dev/dri/renderD{}", rustix::fs::minor(render_node));
        Ok(Rc::new(Self(open(path, OFlags::RDWR | OFlags::CLOEXEC, Mode::empty())?)))
    }
}

pub struct Timeline {
    drm: Rc<Drm>,
    handle: u32,
}

impl Timeline {
    pub fn import(drm: &Rc<Drm>, fd: &OwnedFd) -> io::Result<Self> {
        let mut arg = Handle { handle: 0, flags: 0, fd: fd.as_raw_fd(), pad: 0 };
        ioctl(drm.0.as_fd(), SYNCOBJ_FD_TO_HANDLE, &mut arg)?;
        Ok(Self { drm: drm.clone(), handle: arg.handle })
    }

    pub fn wait(&self, point: u64) -> io::Result<OwnedFd> {
        let fd = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)?;
        let mut arg = Eventfd { handle: self.handle, flags: 0, point, fd: fd.as_raw_fd(), pad: 0 };
        ioctl(self.drm.0.as_fd(), SYNCOBJ_EVENTFD, &mut arg)?;
        Ok(fd)
    }

    pub fn signal(&self, point: u64) {
        let (handle, mut point) = (self.handle, point);
        let mut arg = TimelineArray {
            handles: &handle as *const u32 as u64,
            points: &mut point as *mut u64 as u64,
            count: 1,
            flags: 0,
        };
        if let Err(e) = ioctl(self.drm.0.as_fd(), SYNCOBJ_TIMELINE_SIGNAL, &mut arg) {
            log::warn!("cannot signal release point: {e}");
        }
    }
}

impl Drop for Timeline {
    fn drop(&mut self) {
        let mut arg = [self.handle, 0];
        let _ = ioctl(self.drm.0.as_fd(), SYNCOBJ_DESTROY, &mut arg);
    }
}

pub fn realtime() {
    const SCHED_RESET_ON_FORK: libc::c_int = 0x4000_0000;
    let param = libc::sched_param { sched_priority: 1 };
    match unsafe { libc::sched_setscheduler(0, libc::SCHED_RR | SCHED_RESET_ON_FORK, &param) } {
        0 => log::info!("running with realtime priority"),
        _ => log::info!("no realtime priority: {}", io::Error::last_os_error()),
    }
    unsafe { libc::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_CLEAR_ALL, 0, 0, 0) };
    let none = rustix::thread::CapabilitySet::empty();
    let sets = rustix::thread::CapabilitySets { effective: none, permitted: none, inheritable: none };
    if let Err(e) = rustix::thread::set_capabilities(None, sets) {
        log::warn!("cannot drop capabilities: {e}");
    }
}

pub fn notify(state: &str) {
    use std::os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixDatagram},
    };
    let Some(path) = std::env::var_os("NOTIFY_SOCKET") else { return };
    let address = match path.as_encoded_bytes().strip_prefix(b"@") {
        Some(name) => SocketAddr::from_abstract_name(name),
        None => SocketAddr::from_pathname(&path),
    };
    let sent = UnixDatagram::unbound().and_then(|s| s.send_to_addr(state.as_bytes(), &address?));
    if let Err(e) = sent {
        log::warn!("cannot notify systemd: {e}");
    }
}

pub fn inherit(command: &mut std::process::Command, fds: [std::os::fd::RawFd; 2]) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(move || {
            for fd in fds {
                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
}

pub fn die_with_parent(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| match libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) {
            0 => Ok(()),
            _ => Err(io::Error::last_os_error()),
        });
    }
}
