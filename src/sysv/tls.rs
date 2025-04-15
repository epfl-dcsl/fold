use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use core::ptr::{copy_nonoverlapping, null_mut};

use rustix::mm::{mmap_anonymous, MapFlags, ProtFlags};

use crate::module::Module;

const TCB_HEAD_SIZE: usize = 704;

pub struct SysvTls {}

impl SysvTls {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvTls {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvTls {
    fn name(&self) -> &'static str {
        "sysv-tls"
    }

    fn process_manifold(
        &mut self,
        _manifold: &mut crate::manifold::Manifold,
    ) -> Result<(), alloc::boxed::Box<dyn core::fmt::Debug>> {
        log::info!("setting up tls");

        let tls = build_tls(Default::default());
        unsafe { set_fs(tls) };

        Ok(())
    }
}

fn build_tls(offsets: BTreeMap<usize, usize>) -> usize {
    let storage_size = offsets.len() * size_of::<usize>();

    let location = unsafe {
        mmap_anonymous(
            null_mut(),
            TCB_HEAD_SIZE + storage_size,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )
        .unwrap()
    };

    unsafe {
        let bytes = build_tcb_head(location as usize, storage_size);
        copy_nonoverlapping(bytes.as_ptr(), location as *mut u8, bytes.len());
    }

    location as usize
}

fn build_tcb_head(addr: usize, storage_size: usize) -> Vec<u8> {
    let mut block = Vec::with_capacity(storage_size + TCB_HEAD_SIZE);

    // Zero-out storage
    block.extend(core::iter::repeat_n(0, storage_size));

    // Build a minimal tcbheat
    block.extend(&addr.to_le_bytes()); // tcb
    block.extend(&0_u64.to_le_bytes()); // dtv
    block.extend(&addr.to_le_bytes()); // thread pointer
    block.extend(&0_u32.to_le_bytes()); // multiple threads
    block.extend(&0_u32.to_le_bytes()); // gscope_flag
    block.extend(&0_u64.to_le_bytes()); // sysinfo
    block.extend(&0xDEADBEEF_u64.to_le_bytes()); // stack guard
    block.extend(&0xDEADBEEF_u64.to_le_bytes()); // pointer guard

    // Padding
    block.extend(core::iter::repeat_n(0, block.capacity() - block.len()));

    block
}

unsafe fn set_fs(addr: usize) {
    log::trace!("Set fs register to 0x{:x}", addr);
    let syscall_number: u64 = 158; // arch_prctl syscall
    let arch_set_fs: u64 = 0x1002; // set FS

    asm!(
        "syscall",
        inout("rax") syscall_number => _,
        in("rdi") arch_set_fs,
        in("rsi") addr,
        lateout("rcx") _, lateout("r11") _,
    );
}
