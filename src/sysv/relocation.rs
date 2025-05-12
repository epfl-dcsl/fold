use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::cell::LazyCell;
use core::str::FromStr;

use goblin::elf::reloc::{R_X86_64_64, R_X86_64_COPY, R_X86_64_JUMP_SLOT, R_X86_64_RELATIVE, *};
use goblin::elf::section_header::SHT_RELA;
use goblin::elf::sym::STB_WEAK;
use goblin::elf64::reloc::{self, Rela};

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::sysv::error::SysvError;
use crate::sysv::loader::SYSV_LOADER_BASE_ADDR;
use crate::{Handle, Object, Section};

macro_rules! apply_reloc {
    ($addr:expr, $value:expr, $type:ty) => {
        let value = $value;
        // log::trace!("Relocate {:x?} to 0x{:x?}", $addr, value);
        unsafe { core::ptr::write_unaligned($addr as *mut $type, value as $type) };
    };
}

// ———————————————————————————————— Library relocation ————————————————————————————————— //

#[derive(Default)]
pub struct SysvReloc {
    relocated: Vec<Handle<Object>>,
}

impl SysvReloc {
    pub fn new() -> Self {
        Self::default()
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

impl Module for SysvReloc {
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
                        process_reloc(obj, section, manifold)?;
                    }
                }
            }
        }
        Ok(())
    }
}

// ———————————————————————————————— Relocation ————————————————————————————————— //

fn process_reloc(
    obj: &Object,
    section: &Section,
    manifold: &Manifold,
) -> Result<(), Box<dyn core::fmt::Debug>> {
    log::info!(
        "Process relocation of section {:?} for object {}...",
        section.get_display_name(manifold).unwrap_or_default(),
        obj.display_path()
    );

    let base = obj
        .shared
        .get(SYSV_LOADER_BASE_ADDR)
        .copied()
        .ok_or(SysvError::RelaSectionWithoutVirtualAdresses)? as *mut u8;

    let b = base as i64;
    let _got: i64 = obj
        .find_symbol(
            CString::from_str("_GLOBAL_OFFSET_TABLE_")
                .unwrap()
                .as_c_str(),
            manifold,
        )
        .map(|entry| b + entry.1.st_value as i64)
        .unwrap_or_default();

    'rela: for rela in ElfItemIterator::<Rela>::from_section(section) {
        let addr = unsafe { base.add(rela.r_offset as usize) };
        let r#type = reloc::r_type(rela.r_info);
        let sym = reloc::r_sym(rela.r_info);

        let a = rela.r_addend;

        // Lazily computed to avoid overhead if the relocation does not use the symbol's address.
        // Also, neat trick to warn for not found symbols only when the value is actually used.
        let s = LazyCell::new(|| {
            let (s, name) = 's: {
                // Get the symbol's name
                let Some(name) = section
                    .get_linked_section(manifold)
                    .ok()
                    .and_then(|s| s.as_dynamic_symbol_table().ok())
                    .and_then(|s| s.get_symbol_name(sym as usize, manifold).ok())
                else {
                    break 's (None, None);
                };

                // Ignore empty symbols
                if name.is_empty() {
                    break 's (None, Some(name));
                }

                // Find the related symbol in the loaded objects
                (
                    manifold
                        .find_symbol(name, section.obj)
                        .map(|(section, sym)| {
                            manifold[section.obj]
                                .shared
                                .get(SYSV_LOADER_BASE_ADDR)
                                .copied()
                                .unwrap() as i64
                                + sym.st_value as i64
                        })
                        .ok(),
                    Some(name),
                )
            };

            s.unwrap_or_else(|| {
                log::warn!("Unable to locate symbol {name:?}");
                0
            })
        });

        // See https://web.archive.org/web/20250319095707/https://gitlab.com/x86-psABIs/x86-64-ABI
        match r#type {
            R_X86_64_NONE => {}
            R_X86_64_64 => {
                apply_reloc!(addr, *s + a, u64);
            }
            R_X86_64_COPY => {
                let name = section
                    .get_linked_section(manifold)?
                    .as_dynamic_symbol_table()?
                    .get_symbol_name(sym as usize, manifold)?;

                'find_symbol: for (_, lib_obj) in
                    manifold.objects.enumerate().filter(|s| s.0 != section.obj)
                {
                    let Ok((_, lib_sym)) = lib_obj.find_symbol(name, manifold) else {
                        continue;
                    };

                    let src = *lib_obj.shared.get(SYSV_LOADER_BASE_ADDR).unwrap_or(&0)
                        + lib_sym.st_value as usize;

                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            src as *const u8,
                            addr,
                            lib_sym.st_size as usize,
                        );
                        break 'find_symbol;
                    }
                }
            }
            R_X86_64_JUMP_SLOT => {
                apply_reloc!(addr, *s, u64);
            }
            R_X86_64_GLOB_DAT => {
                if (rela.r_info & (1 << STB_WEAK as u64)) != 0 {
                    apply_reloc!(addr, *s, u64);
                }

                let name = section
                    .get_linked_section(manifold)?
                    .as_dynamic_symbol_table()?
                    .get_symbol_name(sym as usize, manifold)?;

                for (_, lib_obj) in manifold.objects.enumerate() {
                    let Ok((container, lib_sym)) = lib_obj.find_dynamic_symbol(name, manifold)
                    else {
                        continue;
                    };
                    if lib_sym.st_value == 0 {
                        continue;
                    }

                    let start = lib_sym.st_value as usize + container.offset - container.addr;

                    let lib_content = lib_obj
                        .shared
                        .get(SYSV_LOADER_BASE_ADDR)
                        .copied()
                        .unwrap_or_default();

                    apply_reloc!(addr, lib_content + start, u64);
                    continue 'rela;
                }
            }
            R_X86_64_32 | R_X86_64_32S => {
                apply_reloc!(addr, *s + a, u32);
            }
            R_X86_64_16 => {
                apply_reloc!(addr, *s + a, u16);
            }
            R_X86_64_8 => {
                apply_reloc!(addr, *s + a, u8);
            }
            R_X86_64_RELATIVE => {
                apply_reloc!(addr, b + a, u64);
            }
            R_X86_64_IRELATIVE => {
                let code: extern "C" fn() -> i64 = unsafe { core::mem::transmute(b + a) };
                apply_reloc!(addr, code(), i64);
            }
            _ => panic!("unknown rela type 0x{:x}", r#type),
        };
    }

    Ok(())
}
