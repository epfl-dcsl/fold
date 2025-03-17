use core::fmt;

use rustix::fd::OwnedFd;
use rustix::{fs, mm, path};

const S_IFMT: u32 = 0xf000;
const S_IFDIR: u32 = 0x4000;

pub struct Mapping {
    /// Mapped region, owned by the mapping
    bytes: &'static [u8],
    /// File descriptor, if backed by a file
    _fd: Option<OwnedFd>,
}

pub struct MappingMut {
    /// Mapped region, owned by the mapping
    bytes: &'static mut [u8],
}

impl Mapping {
    pub(crate) unsafe fn new(ptr: *const u8, len: usize, fd: Option<OwnedFd>) -> Self {
        Self {
            bytes: core::slice::from_raw_parts(ptr, len),
            _fd: fd,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
}

impl MappingMut {
    pub(crate) unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        Self {
            bytes: core::slice::from_raw_parts_mut(ptr, len),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes
    }
}

/// Open a file in read only
pub fn open_file_ro<P: path::Arg>(path: P) -> Result<OwnedFd, ()> {
    let fd = fs::open(path, fs::OFlags::RDONLY, fs::Mode::empty()).map_err(|_| ())?;
    let stat = fs::fstat(&fd).map_err(|_| ())?;

    if (stat.st_mode & S_IFMT) == S_IFDIR {
        // It is a directory
        Err(())
    } else {
        // If it not a directory, most likely a file
        // See `man stat`, or https://linux.die.net/man/2/stat
        Ok(fd)
    }
}

pub fn map_file(fd: OwnedFd) -> Mapping {
    let stat = fs::fstat(&fd).expect("Could not retrieve file size");
    let len = stat.st_size as usize;

    // Safety: we let the OS choose the mapping, thus this wont affect existing mappings
    unsafe {
        let ptr = mm::mmap(
            core::ptr::null_mut(),
            len,
            mm::ProtFlags::READ,
            mm::MapFlags::PRIVATE,
            &fd,
            0,
        )
        .expect("mmap failed");

        Mapping::new(ptr as *mut u8, len, Some(fd))
    }
}

// ———————————————————————————————— Display ————————————————————————————————— //

impl fmt::Debug for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mapping")
            .field("addr", &self.bytes.as_ptr())
            .field("len", &self.bytes().len())
            .finish()
    }
}
