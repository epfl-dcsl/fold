use alloc::boxed::Box;
use alloc::vec::Vec;

use fold::module::Module;
use syscalls::{syscall, Sysno};

// Constants from <linux/seccomp.h>, <linux/filter.h>, and <linux/prctl.h>
const SECCOMP_SET_MODE_FILTER: i32 = 1;
const PR_SET_NO_NEW_PRIVS: i32 = 38;

const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_KILL: u32 = 0x0000_0000;

const SYS_WRITE: u32 = 1;
const SYS_EXIT: u32 = 60;

const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JEQ: u16 = 0x10;
const BPF_RET: u16 = 0x06;
const BPF_K: u16 = 0x00;

#[repr(C)]
struct sock_filter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[repr(C)]
struct sock_fprog {
    len: u16,                 // Number of BPF instructions
    filter: *mut sock_filter, // Pointer to array of BPF instructions
}

#[derive(Debug)]
struct SeccompError;

impl From<SeccompError> for Box<dyn core::fmt::Debug> {
    fn from(value: SeccompError) -> Self {
        Box::new(value)
    }
}

// Generate a filter that allows only the listed syscalls
fn build_seccomp_filter(allowed_syscalls: &[u32]) -> Vec<sock_filter> {
    let mut filters = Vec::new();

    // 1. Load syscall number into accumulator (A)
    filters.push(sock_filter {
        code: BPF_LD | BPF_W | BPF_ABS,
        jt: 0,
        jf: 0,
        k: 0, // seccomp loads syscall number at offset 0
    });

    // 2. Add a JEQ jump for each allowed syscall
    for &num in allowed_syscalls {
        filters.push(sock_filter {
            code: BPF_JMP | BPF_JEQ | BPF_K,
            jt: 0,
            jf: 1, // if not match, skip next RET ALLOW
            k: num,
        });

        // RET ALLOW (if syscall matched)
        filters.push(sock_filter {
            code: BPF_RET | BPF_K,
            jt: 0,
            jf: 0,
            k: SECCOMP_RET_ALLOW,
        });
    }

    // 3. Default action: kill syscall
    filters.push(sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0,
        jf: 0,
        k: SECCOMP_RET_KILL,
    });

    filters
}

pub struct Seccomp;

impl Module for Seccomp {
    fn name(&self) -> &'static str {
        "seccomp"
    }

    fn process_manifold(
        &mut self,
        _manifold: &mut fold::manifold::Manifold,
    ) -> Result<(), alloc::boxed::Box<dyn core::fmt::Debug>> {
        // Combine filters for write and exit
        let mut filters = build_seccomp_filter(&[SYS_WRITE, SYS_EXIT]);

        let mut prog = sock_fprog {
            len: filters.len() as u16,
            filter: filters.as_mut_ptr(),
        };
        unsafe {
            // Requiered by SECCOMP_SET_MODE_FILTER
            syscall!(Sysno::prctl, PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)
                .map(|_| ())
                .map_err(|_| Box::from(SeccompError))?;

            // Install the filter using seccomp syscall
            syscall!(
                Sysno::seccomp,
                SECCOMP_SET_MODE_FILTER,
                0,
                &mut prog as *mut _ as usize
            )
            .map(|_| ())
            .map_err(|_| Box::from(SeccompError))
        }
    }
}
