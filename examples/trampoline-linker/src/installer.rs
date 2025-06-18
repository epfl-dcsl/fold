use core::ffi::c_void;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use goblin::elf::reloc::*;
use goblin::elf::section_header::SHT_RELA;
use goblin::elf64::reloc::{self, Rela};

use fold::elf::ElfItemIterator;
use fold::manifold::Manifold;
use fold::module::Module;
use fold::object::section::SectionT;
use fold::sysv::error::SysvError;
use fold::sysv::loader::SYSV_LOADER_BASE_ADDR;
use fold::{Handle, Object, Section, println};
use rustix::mm::{self, MprotectFlags};

macro_rules! apply_reloc {
    ($addr:expr, $value:expr, $type:ty) => {
        let value = $value;
        // log::trace!("Relocate {:x?} to 0x{:x?}", $addr, value);
        unsafe { core::ptr::write_unaligned($addr as *mut $type, value as $type) };
    };
}

// ———————————————————————————————— Library relocation ————————————————————————————————— //

type HookMapping = BTreeMap<String, fn()>;

#[derive(Default)]
pub struct TrampolineReloc {
    relocated: Vec<Handle<Object>>,
    hooks: HookMapping,
}

impl TrampolineReloc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_hook(mut self, symbol: &str, target: fn()) -> Self {
        self.hooks.insert(symbol.to_owned(), target);
        self
    }

    fn setup_hooks(&mut self, hook: fn(), hook_target: u64) {
        let hook = (hook as *const ()) as u64;
        let page = hook & !0xFFF;

        unsafe {
            mm::mprotect(
                page as *mut c_void,
                4096,
                MprotectFlags::READ | MprotectFlags::EXEC | MprotectFlags::WRITE,
            )
            .unwrap();
        }

        let mov = hook + 3;
        unsafe { core::ptr::write_unaligned(mov as *mut u64, hook_target) }

        unsafe {
            mm::mprotect(
                page as *mut c_void,
                4096,
                MprotectFlags::READ | MprotectFlags::EXEC,
            )
            .unwrap();
        }
    }

    fn process_reloc(
        &mut self,
        obj: &Object,
        section: &Section,
        manifold: &Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        log::info!(
            "Process relocation of section {:?} for object {}...",
            section.get_display_name(),
            obj.display_path()
        );

        let base = obj
            .shared
            .get(SYSV_LOADER_BASE_ADDR)
            .copied()
            .ok_or(SysvError::RelaSectionWithoutVirtualAdresses)? as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr: *mut u8 = unsafe { base.add(rela.r_offset as usize) };
            let r#type = reloc::r_type(rela.r_info);

            if r#type != R_X86_64_JUMP_SLOT {
                continue;
            }

            let sym = reloc::r_sym(rela.r_info);

            // Lazily computed to avoid overhead if the relocation does not use the symbol's address.
            // Also, neat trick to warn for not found symbols only when the value is actually used.
            // Get the symbol's name
            let Some(name) = section
                .get_linked_section(manifold)
                .ok()
                .and_then(|s| s.as_dynamic_symbol_table().ok())
                .and_then(|s| s.get_symbol_name(sym as usize, manifold).ok())
            else {
                continue;
            };

            let Some(target) = self.hooks.get(name.to_str().unwrap_or_default()) else {
                continue;
            };

            // See https://web.archive.org/web/20250319095707/https://gitlab.com/x86-psABIs/x86-64-ABI
            println!("found a matching symbol !");
            let value = unsafe { *(addr as *const u64) };
            apply_reloc!(addr, (*target as *const ()) as u64, u64);
            self.setup_hooks(*target, value);
        }

        Ok(())
    }
}

fn add_deps(obj: &Object, manifold: &Manifold) -> Vec<Handle<Object>> {
    let mut queue = Vec::new();
    for dep in obj.dependencies.iter() {
        queue.push(*dep);
        queue.extend(add_deps(&manifold[*dep], manifold));
    }
    queue
}

impl Module for TrampolineReloc {
    fn name(&self) -> &'static str {
        "sysv-reloc-lib"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let mut tree: Vec<Handle<Object>> = Vec::new();

        tree.push(obj);
        tree.extend(add_deps(&manifold[obj], manifold));

        for dep in tree.into_iter().rev() {
            if !self.relocated.contains(&dep) {
                self.relocated.push(dep);
                let obj = manifold.objects.get(dep).unwrap();
                for section in obj.sections.iter() {
                    let section = &manifold[*section];
                    if section.tag == SHT_RELA {
                        self.process_reloc(obj, section, manifold)?;
                    }
                }
            }
        }

        Ok(())
    }
}

// ———————————————————————————————— Relocation ————————————————————————————————— //
