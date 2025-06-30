use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::fmt::Debug;

use goblin::elf::dynamic::DT_NEEDED;
use goblin::elf64::dynamic::Dyn;
use log::trace;
use rustix::fs;

use crate::arena::Handle;
use crate::elf::{ElfItemIterator, Section};
use crate::file;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::{Object, SectionT};
use crate::share_map::ShareMapKey;
use crate::sysv::error::SysvError;

/// Returns the name of all dependencies of a given object
fn read_deps(sec: &Section, manifold: &Manifold) -> Result<Vec<CString>, SysvError> {
    let mut deps = Vec::new();

    let linked_dynstr = sec.get_linked_section(manifold)?.as_string_table()?;

    // Add all entries marked with DT_NEEDED to the object's dependencies
    deps.append(
        &mut ElfItemIterator::<Dyn>::from_section(sec)
            .filter(|e| e.d_tag == DT_NEEDED)
            .map(|e| e.d_val)
            .map(|idx| linked_dynstr.get_symbol(idx as usize).map(CStr::to_owned))
            .collect::<Result<Vec<_>, _>>()?,
    );

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

pub const SYSV_COLLECTOR_SEARCH_PATHS_KEY: ShareMapKey<Vec<String>> =
    ShareMapKey::new("sysv_collector_search_paths");
pub const SYSV_COLLECTOR_REMAP_KEY: ShareMapKey<BTreeMap<String, Option<CString>>> =
    ShareMapKey::new("sysv_collector_map");
pub const SYSV_COLLECTOR_RESULT_KEY: ShareMapKey<Vec<SysvCollectorEntry>> =
    ShareMapKey::new("sysv_collector_result");

pub struct SysvCollector;

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_section(
        &mut self,
        hsec: Handle<Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // No recursion is needed for collecting dependencies; object needed by the executable are loaded into the manifold,
        // then the collector is invoked on them, etc.

        // Fetches the already loaded depencencies
        let section = &manifold[hsec];
        let hobj = section.obj;
        let mut deps: Vec<SysvCollectorEntry> = manifold
            .shared
            .get(SYSV_COLLECTOR_RESULT_KEY)
            .cloned()
            .unwrap_or_default();

        // Compute the dependencies of the current object, and removes the ones already found
        let new_deps = read_deps(section, manifold)?
            .into_iter()
            .filter(|n| deps.iter().all(|d| d.name != *n))
            .collect::<Vec<_>>();

        // Loads all the newly found dependenciesadd_elf
        for filename in new_deps {
            let path_lib = manifold
                .shared
                .get(SYSV_COLLECTOR_SEARCH_PATHS_KEY)
                .expect("Search paths not set")
                .iter()
                .map(|p| format!("{}/{}", p, filename.to_str().unwrap()))
                .find(|p| fs::stat(p.as_str()).is_ok())
                .ok_or_else(|| SysvError::DependencyNotFound(filename.clone()))?;

            let file_fd = file::open_file_ro(path_lib.as_str()).expect("Target is not a file");

            let file = file::map_file(file_fd);
            let obj = manifold.add_elf_file(file, filename.clone());

            manifold[hobj].dependencies.push(obj);

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
pub struct SysvRemappingCollector;

impl SysvRemappingCollector {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Module for SysvRemappingCollector {
    fn name(&self) -> &'static str {
        "sysv-remapping-collector"
    }

    fn process_section(
        &mut self,
        hsec: Handle<Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // No recursion is needed for collecting dependencies; object needed by the executable are loaded into the manifold,
        // then the collector is invoked on them, etc.

        // Fetches the already loaded depencencies
        let sec = &manifold[hsec];
        let hobj = sec.obj;
        let mut deps: Vec<SysvCollectorEntry> = manifold
            .shared
            .get(SYSV_COLLECTOR_RESULT_KEY)
            .cloned()
            .unwrap_or_default();
        trace!("[{}] Collecting from obj", manifold[hobj].display_path());
        trace!(
            "[{}] Initial deps: {:?}",
            manifold[hobj].display_path(),
            deps
        );

        let empty_map = BTreeMap::new();
        let map = manifold
            .shared
            .get(SYSV_COLLECTOR_REMAP_KEY)
            .unwrap_or(&empty_map);

        // Compute and remap the dependencies of the current object
        let new_deps = read_deps(sec, manifold)?.into_iter().filter_map(|d| {
            let Ok(dstr) = d.to_str() else { return Some(d) };

            let entry = map.iter().find(|(k, _)| dstr.starts_with(*k));

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

        trace!(
            "[{}] New deps: {:?}",
            manifold[hobj].display_path(),
            new_deps
        );

        // Loads all the newly found dependencies
        for filename in new_deps {
            let path_lib = manifold
                .shared
                .get(SYSV_COLLECTOR_SEARCH_PATHS_KEY)
                .expect("Search paths not set")
                .iter()
                .map(|p| format!("{}/{}", p, filename.to_str().unwrap()))
                .find(|p| fs::stat(p.as_str()).is_ok())
                .ok_or_else(|| SysvError::DependencyNotFound(filename.clone()))?;

            let file_fd = file::open_file_ro(path_lib.as_str()).expect("Target is not a file");

            let file = file::map_file(file_fd);
            let obj = manifold.add_elf_file(file, filename.clone());

            manifold[hobj].dependencies.push(obj);

            deps.push(SysvCollectorEntry {
                name: filename,
                obj,
            });
        }

        manifold.shared.insert(SYSV_COLLECTOR_RESULT_KEY, deps);

        Ok(())
    }
}
