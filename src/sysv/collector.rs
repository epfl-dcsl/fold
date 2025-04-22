use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::fmt::Debug;
use core::str::FromStr;
use rustix::fs;

use goblin::elf::dynamic::DT_NEEDED;
use goblin::elf::section_header::SHT_DYNAMIC;
use goblin::elf64::dynamic::Dyn;
use log::trace;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::share_map::ShareMapKey;
use crate::sysv::error::SysvError;
use crate::{file, Handle, Object};

/// Returns the name of all dependencies of a given object
fn read_deps(obj: &Object, manifold: &Manifold) -> Result<Vec<CString>, SysvError> {
    let mut deps = Vec::new();

    // Iterates over all dynamic sections
    for sec in obj
        .sections
        .iter()
        .map(|sec| &manifold.sections[*sec])
        .filter(|sec| sec.tag == SHT_DYNAMIC)
    {
        let linked_dynstr = sec.get_linked_section(manifold)?.as_string_table()?;

        // Add all entries marked with DT_NEEDED to the object's dependencies
        deps.append(
            &mut ElfItemIterator::<Dyn>::from_section(sec)
                .filter(|e| e.d_tag == DT_NEEDED)
                .map(|e| e.d_val)
                .map(|idx| linked_dynstr.get_symbol(idx as usize).map(CStr::to_owned))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }

    trace!("[{}] Found deps: {:?}", obj.display_path(), deps);
    Ok(deps)
}

#[derive(Clone)]
pub struct SysvCollectorEntry {
    /// Filename of the dependency
    pub name: CString,
    /// Handle to the ELF file of the dependency
    pub obj: Handle<Object>,
}

impl Debug for SysvCollectorEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.name)
    }
}

pub const SYSV_COLLECTOR_RESULT_KEY: ShareMapKey<Vec<SysvCollectorEntry>> =
    ShareMapKey::new("sysv_collector");

pub struct SysvCollector {}

impl SysvCollector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // No recursion is needed for collecting dependencies; object needed by the executable are loaded into the manifold,
        // then the collector is invoked on them, etc.

        // Fetches the already loaded depencencies
        let obj = &manifold[obj];
        let mut deps: Vec<SysvCollectorEntry> = manifold
            .shared
            .get(SYSV_COLLECTOR_RESULT_KEY)
            .cloned()
            .unwrap_or_default();
        trace!("[{}] Collecting from obj", obj.display_path());
        trace!("[{}] Initial deps: {:?}", obj.display_path(), deps);

        // Compute the dependencies of the current object, and removes the ones already found
        let new_deps = read_deps(obj, manifold)?
            .into_iter()
            .filter(|n| deps.iter().all(|d| d.name != *n))
            .collect::<Vec<_>>();
        trace!("[{}] New deps: {:?}", obj.display_path(), new_deps);

        // Loads all the newly found dependencies
        for filename in new_deps {
            let path_lib = manifold
                .search_paths
                .iter()
                .map(|p| format!("{}/{}", p, filename.to_str().unwrap()))
                .find(|p| fs::stat(p.as_str()).is_ok())
                .ok_or_else(|| SysvError::DependencyNotFound(filename.clone()))?;

            let file_fd = file::open_file_ro(path_lib.as_str()).expect("Target is not a file");

            let file = file::map_file(file_fd);
            let obj = manifold.add_elf_file(file, filename.clone());

            manifold.objects.get_mut(obj).unwrap().is_lib = true;

            deps.push(SysvCollectorEntry {
                name: filename,
                obj,
            });
        }

        manifold.shared.insert(SYSV_COLLECTOR_RESULT_KEY, deps);

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SysvRemappingCollector {
    map: BTreeMap<String, Option<CString>>,
}

impl SysvRemappingCollector {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn replace(mut self, old_deps: &str, new_deps: &str) -> Self {
        self.map
            .insert(old_deps.into(), Some(CString::from_str(new_deps).unwrap()));
        self
    }

    pub fn replace_multiple(self, entries: &[(&str, &str)]) -> Self {
        entries
            .iter()
            .fold(self, |acc, (old, new)| acc.replace(old, new))
    }

    pub fn drop(mut self, deps: &str) -> Self {
        self.map.insert(deps.into(), None);
        self
    }

    pub fn drop_multiple(self, entries: &[&str]) -> Self {
        entries.iter().fold(self, |acc, deps| acc.drop(deps))
    }
}

impl Module for SysvRemappingCollector {
    fn name(&self) -> &'static str {
        "sysv-remapping-collector"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // No recursion is needed for collecting dependencies; object needed by the executable are loaded into the manifold,
        // then the collector is invoked on them, etc.

        // Fetches the already loaded depencencies
        let obj = &manifold[obj];
        let mut deps: Vec<SysvCollectorEntry> = manifold
            .shared
            .get(SYSV_COLLECTOR_RESULT_KEY)
            .cloned()
            .unwrap_or_default();
        trace!("[{}] Collecting from obj", obj.display_path());
        trace!("[{}] Initial deps: {:?}", obj.display_path(), deps);

        // Compute and remap the dependencies of the current object
        let new_deps = read_deps(obj, manifold)?.into_iter().filter_map(|d| {
            let Ok(dstr) = d.to_str() else { return Some(d) };

            let entry = self.map.iter().find(|(k, _)| dstr.starts_with(*k));

            match entry {
                Some((_, val)) => val.clone(),
                None => Some(d),
            }
        });

        // Filter out already found dependencies
        let new_deps = new_deps
            .into_iter()
            .filter(|n| deps.iter().all(|d| d.name != *n))
            .collect::<Vec<_>>();

        trace!("[{}] New deps: {:?}", obj.display_path(), new_deps);

        // Loads all the newly found dependencies
        for filename in new_deps {
            let path_lib = manifold
                .search_paths
                .iter()
                .map(|p| format!("{}/{}", p, filename.to_str().unwrap()))
                .find(|p| fs::stat(p.as_str()).is_ok())
                .ok_or_else(|| SysvError::DependencyNotFound(filename.clone()))?;

            let file_fd = file::open_file_ro(path_lib.as_str()).expect("Target is not a file");

            let file = file::map_file(file_fd);
            let obj = manifold.add_elf_file(file, filename.clone());

            deps.push(SysvCollectorEntry {
                name: filename,
                obj,
            });
        }

        manifold.shared.insert(SYSV_COLLECTOR_RESULT_KEY, deps);

        Ok(())
    }
}
