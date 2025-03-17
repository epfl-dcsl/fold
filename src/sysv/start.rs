use alloc::vec::Vec;
use core::arch::asm;

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

    fn process_object(&mut self, obj: Handle<crate::Object>, manifold: &mut Manifold) {
        let offset = manifold.pie_load_offset.unwrap_or(0);

        let entry = manifold.objects[obj].header().e_entry + offset as u64;

        log::info!(
            "{offset:x}, {:x}, {entry:x}",
            manifold.objects.get(obj).unwrap().header().e_entry
        );

        let stack = build_stack();

        unsafe {
            // set_fs(self.tls.get_fs());

            // log::info!("Calling {} initializer(s)", self.init_array.len());
            // for f in &self.init_array {
            //     log::debug!("Calling {:p}", *f);
            //     f(argc, argv, envp);
            // }

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

pub fn build_stack() -> Vec<u64> {
    let mut stack = Vec::new();
    let null = 0; // The null byte

    // Args
    stack.push(0); // argc, none for now
                   // TODO: argv
    stack.push(null);

    // Env
    // TODO: add env vars
    stack.push(null);

    // Auxv
    // TODO: add auxv
    stack.push(null);
    stack.push(null);

    stack
}
