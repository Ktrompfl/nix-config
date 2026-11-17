#![allow(unsafe_code)]

use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};
use std::{
    cell::Cell,
    ffi::{c_int, c_void},
    io, mem,
    os::fd::OwnedFd,
    ptr,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};
use wire::{DISPLAY, Error, wl_display, wl_shm};

const MAX_CLIENT_POOLS: usize = 256;
const MAX_SIZE: i32 = 16384;

struct Slot {
    start: AtomicUsize,
    len: AtomicUsize,
    faulted: AtomicBool,
}

static SLOTS: [Slot; 1024] =
    [const { Slot { start: AtomicUsize::new(0), len: AtomicUsize::new(0), faulted: AtomicBool::new(false) } }; 1024];

pub fn install_sigbus_handler() -> io::Result<()> {
    unsafe {
        let mut action: libc::sigaction = mem::zeroed();
        action.sa_sigaction = on_sigbus as extern "C" fn(c_int, *mut libc::siginfo_t, *mut c_void) as usize;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
        libc::sigemptyset(&mut action.sa_mask);
        if libc::sigaction(libc::SIGBUS, &action, ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

extern "C" fn on_sigbus(signal: c_int, info: *mut libc::siginfo_t, _: *mut c_void) {
    let address = unsafe { (*info).si_addr() } as usize;
    for slot in &SLOTS {
        let (start, len) = (slot.start.load(SeqCst), slot.len.load(SeqCst));
        if start == 0 || !(start..start + len).contains(&address) {
            continue;
        }
        let flags = libc::MAP_FIXED | libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
        if unsafe { libc::mmap(start as *mut c_void, len, libc::PROT_READ | libc::PROT_WRITE, flags, -1, 0) }
            != libc::MAP_FAILED
        {
            slot.faulted.store(true, SeqCst);
            return;
        }
    }
    unsafe { libc::signal(signal, libc::SIG_DFL) };
}

pub struct Pool {
    fd: OwnedFd,
    ptr: Cell<*mut u8>,
    len: Cell<usize>,
    writable: Cell<bool>,
    slot: usize,
    count: Rc<Cell<usize>>,
}

impl Pool {
    pub fn new(shm: u32, fd: OwnedFd, size: i32, count: Rc<Cell<usize>>) -> Result<Self, Error> {
        if size <= 0 {
            return Err(Error::new(shm, wl_shm::error::INVALID_STRIDE, format!("invalid pool size {size}")));
        }
        let full = |m| Error::new(DISPLAY.id, wl_display::error::NO_MEMORY, m);
        if count.get() >= MAX_CLIENT_POOLS {
            return Err(full("too many shm pools for this client"));
        }
        let slot = SLOTS.iter().position(|s| s.start.load(SeqCst) == 0).ok_or_else(|| full("too many shm pools"))?;
        let (ptr, len, writable) = (Cell::new(ptr::null_mut()), Cell::new(0), Cell::new(false));
        count.set(count.get() + 1);
        let pool = Self { fd, ptr, len, writable, slot, count };
        pool.map(size).map_err(|e| Error::new(shm, wl_shm::error::INVALID_FD, e))?;
        Ok(pool)
    }

    pub fn resize(&self, size: i32) -> Result<(), (u32, String)> {
        match usize::try_from(size) {
            Ok(len) if len >= self.len.get() => self.map(size).map_err(|e| (wl_shm::error::INVALID_FD, e)),
            _ => Err((wl_shm::error::INVALID_STRIDE, format!("pools cannot shrink to {size}"))),
        }
    }

    pub fn faulted(&self) -> bool {
        SLOTS[self.slot].faulted.load(SeqCst)
    }

    fn map(&self, size: i32) -> Result<(), String> {
        let len = usize::try_from(size).ok().filter(|&l| l > 0).ok_or(format!("invalid pool size {size}"))?;
        let map = |prot| unsafe { mmap(ptr::null_mut(), len, prot, MapFlags::SHARED, &self.fd, 0) };
        let (new, writable) = match map(ProtFlags::READ | ProtFlags::WRITE) {
            Ok(new) => (new, true),
            Err(_) => (map(ProtFlags::READ).map_err(|e| e.to_string())?, false),
        };
        self.unmap();
        let slot = &SLOTS[self.slot];
        slot.len.store(len, SeqCst);
        slot.start.store(new as usize, SeqCst);
        self.ptr.set(new.cast());
        self.len.set(len);
        self.writable.set(writable);
        Ok(())
    }

    fn unmap(&self) {
        let old = self.ptr.replace(ptr::null_mut());
        if !old.is_null() {
            SLOTS[self.slot].start.store(0, SeqCst);
            let _ = unsafe { munmap(old.cast(), self.len.get()) };
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.unmap();
        SLOTS[self.slot].faulted.store(false, SeqCst);
        self.count.set(self.count.get() - 1);
    }
}

pub struct ShmBuffer {
    pool: Rc<Pool>,
    offset: usize,
    size: (i32, i32),
    stride: usize,
    opaque: bool,
}

impl ShmBuffer {
    pub fn new(pool: Rc<Pool>, offset: i32, size: (i32, i32), stride: i32, format: u32) -> Result<Self, (u32, String)> {
        let opaque = match format {
            wl_shm::format::ARGB8888 => false,
            wl_shm::format::XRGB8888 => true,
            _ => return Err((wl_shm::error::INVALID_FORMAT, format!("unsupported format {format}"))),
        };
        let (w, h) = size;
        let valid = |v| (1..=MAX_SIZE).contains(&v);
        let fits = valid(w) && valid(h) && offset >= 0 && i64::from(stride) >= i64::from(w) * 4;
        let end = i64::from(offset) + i64::from(stride) * i64::from(h - 1) + i64::from(w) * 4;
        if !fits || end > pool.len.get() as i64 {
            return Err((
                wl_shm::error::INVALID_STRIDE,
                format!("buffer {w}x{h} stride {stride} offset {offset} exceeds pool"),
            ));
        }
        Ok(Self { pool, offset: offset as usize, size, stride: stride as usize, opaque })
    }

    pub fn faulted(&self) -> bool {
        self.pool.faulted()
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    pub fn write_from(&self, pixels: &[u32], pixels_width: usize, x: usize, y: usize) -> bool {
        let (w, h) = (self.size.0 as usize, self.size.1 as usize);
        let fits = x + w <= pixels_width && (y + h) * pixels_width <= pixels.len();
        if !fits || !self.pool.writable.get() {
            return false;
        }
        for row in 0..h {
            let src = pixels[(y + row) * pixels_width + x..][..w].as_ptr();
            let dst = unsafe { self.pool.ptr.get().add(self.offset + row * self.stride) };
            unsafe { ptr::copy_nonoverlapping(src.cast::<u8>(), dst, w * 4) };
        }
        true
    }
}

impl gfx::Source for ShmBuffer {
    fn size(&self) -> (i32, i32) {
        self.size
    }

    fn opaque(&self) -> bool {
        self.opaque
    }

    fn read_row(&self, x: i32, y: i32, out: &mut [u32]) {
        let inside = x >= 0 && (0..self.size.1).contains(&y) && x as usize + out.len() <= self.size.0 as usize;
        if !inside {
            return out.fill(0);
        }
        let at = self.offset + y as usize * self.stride + x as usize * 4;
        unsafe { ptr::copy_nonoverlapping(self.pool.ptr.get().add(at), out.as_mut_ptr().cast(), out.len() * 4) }
    }
}
