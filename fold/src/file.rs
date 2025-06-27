//! Open and store file into the memory.
use core::fmt;

use rustix::fd::OwnedFd;
use rustix::{fs, mm, path};

const S_IFMT: u32 = 0xf000;
const S_IFDIR: u32 = 0x4000;

/// A read-only memory region optionally backed by a file.
pub struct Mapping {
    /// Mapped region, owned by the mapping
   pub(crate) bytes: &'static [u8],
    /// File descriptor, if backed by a file
    pub(crate)fd: Option<OwnedFd>,
}

/// A read-write memory region.
pub struct MappingMut {
    /// Mapped region, owned by the mapping
    bytes: &'static mut [u8],
}

impl Mapping {
    pub(crate) unsafe fn new(ptr: *const u8, len: usize, fd: Option<OwnedFd>) -> Self {
        Self {
            bytes: core::slice::from_raw_parts(ptr, len),
            fd,
        }
    }

    /// Returns the mapping's slice.
    pub fn bytes(&self) -> &'static [u8] {
        self.bytes
    }
}

impl MappingMut {
    pub(crate) unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        Self {
            bytes: core::slice::from_raw_parts_mut(ptr, len),
        }
    }

    /// Returns the mapping's read-only slice.
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Returns the mapping's read-write slice.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes
    }
}

/// Open a file in read only mode. See [`map_file`] to create a corresponding read-only mapping.
pub fn open_file_ro<P: path::Arg + core::marker::Copy + core::fmt::Debug>(
    path: P,
) -> Option<OwnedFd> {
    if fs::stat(path).is_err() {
        panic!("File {:?} doesn't exit", path);
    }
    let fd = fs::open(path, fs::OFlags::RDONLY, fs::Mode::empty()).ok()?;
    let stat = fs::fstat(&fd).ok()?;

    if (stat.st_mode & S_IFMT) == S_IFDIR {
        // It is a directory
        None
    } else {
        // If it not a directory, most likely a file
        // See `man stat`, or https://linux.die.net/man/2/stat
        Some(fd)
    }
}

/// Creates a mapping from an opened file. See [`open_file_ro`] to open files.
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
