use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::asm;
use core::ffi::CStr;

use super::loader::SYSV_LOADER_BASE_ADDR;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::Handle;

pub struct SysvStart {}

impl SysvStart {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvStart {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvStart {
    fn name(&self) -> &'static str {
        "sysv-start"
    }

    fn process_object(
        &mut self,
        obj: Handle<crate::Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let obj = &manifold.objects[obj];
        let offset = obj
            .shared
            .get(SYSV_LOADER_BASE_ADDR)
            .copied()
            .unwrap_or_default();
        let entry = obj.header().e_entry + offset as u64;

        let stack = build_stack(&manifold.env.args);

        unsafe {
            log::info!("Jumping at 0x{:x}...", entry);
            jmp(entry as *const u8, stack.as_ptr(), stack.len() as u64);
        }
    }
}

// ————————————————————————————————— Utils —————————————————————————————————— //

/// The actual jump tot he program entry
#[inline(never)]
unsafe fn jmp(entry_point: *const u8, stack: *const u64, nb_items: u64) -> ! {
    asm!(
        // allocate (qword_count * 8) bytes
        "mov {tmp}, {qword_count}",
        "sal {tmp}, 3",
        "sub rsp, {tmp}",

        "2:",
        // start at i = (n-1)
        "sub {qword_count}, 1",
        // copy qwords to the stack
        "mov {tmp}, QWORD PTR [{stack_contents}+{qword_count}*8]",
        "mov QWORD PTR [rsp+{qword_count}*8], {tmp}",
        // loop if i isn't zero, break otherwise
        "test {qword_count}, {qword_count}",
        "jnz 2b",

        "jmp {entry_point}",

        entry_point = in(reg) entry_point,
        stack_contents = in(reg) stack,
        qword_count = in(reg) nb_items,
        tmp = out(reg) _,
    );

    unreachable!();
}

pub fn build_stack(args: &[&'static CStr]) -> Vec<u64> {
    let mut stack = Vec::new();
    let null = 0; // The null byte

    // Args
    stack.push(args.len() as u64);
    for a in args {
        stack.push(a.as_ptr() as u64);
    }

    // Env
    // TODO: add env vars
    stack.push(null);

    // Auxv
    // TODO: add auxv
    stack.push(null);
    stack.push(null);

    stack
}
