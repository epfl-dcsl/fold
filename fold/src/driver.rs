use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use goblin::elf::program_header::PT_LOAD;
use goblin::elf::section_header::{SHT_FINI_ARRAY, SHT_INIT_ARRAY};

use crate::arena::{Arena, Handle};
use crate::cli::Config;
use crate::env::Env;
use crate::filters::{self, section, segment, ItemFilter, ObjectFilter};
use crate::manifold::Manifold;
use crate::module::Module;
use crate::sysv::collector::{SysvRemappingCollector, SYSV_COLLECTOR_SEARCH_PATHS_KEY};
use crate::sysv::init_array::SysvInitArray;
use crate::sysv::loader::SysvLoader;
use crate::sysv::protect::SysvProtect;
use crate::sysv::relocation::SysvReloc;
use crate::sysv::start::SysvStart;
use crate::sysv::tls::SysvTls;
use crate::{cli, file, Object};

type ModuleRef = Box<dyn Module>;

// —————————————————————————————— Fold Driver ——————————————————————————————— //

pub struct Fold {
    config: Config,
    search_path: Vec<String>,
    phases: Vec<Phase>,
}

struct Phase {
    name: String,
    modules: Arena<ModuleRef>,
    filters: Vec<(ItemFilter, Handle<ModuleRef>)>,
}

pub fn new(env: Env, loader_name: &str) -> Fold {
    log::info!("Hello, world!");
    log::info!("Args: {:?}", &env.args);

    let config = cli::parse(env, loader_name);

    let cwd = if let Some(last_delim) = config.target.to_string_lossy().rfind('/') {
        &config.target.to_string_lossy()[..last_delim]
    } else {
        "."
    };

    log::info!(r#"adding cwd to path: "{cwd}""#);

    let search_path = Vec::from(&[cwd.to_owned()]);

    Fold {
        config,
        search_path,
        phases: Vec::new(),
    }
}

// Return default sysv chain
pub fn default_chain(loader_name: &str, env: Env) -> Fold {
    new(env, loader_name)
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .phase("collect")
        .register(
            SysvRemappingCollector::new()
                .replace("libc.so", "libc.so")
                .drop_multiple(&[
                    "ld-linux-x86-64.so",
                    "libcrypt.so",
                    "libdl.so",
                    "libm.so",
                    "libpthread.so",
                    "libresolv.so",
                    "librt.so",
                    "libutil.so",
                    "libxnet.so",
                ]),
            ObjectFilter::any(),
        )
        .phase("load")
        .register(SysvLoader, segment(PT_LOAD))
        .phase("tls")
        .register(SysvTls, ItemFilter::ManifoldFilter)
        .phase("relocation")
        .register(
            SysvReloc::new(),
            ObjectFilter {
                mask: filters::ObjectMask::Any, // TODO: match only elf
                os_abi: 0,
                elf_type: 0,
            },
        )
        .phase("protect")
        .register(SysvProtect, segment(PT_LOAD))
        .phase("init array")
        .register(SysvInitArray, section(SHT_INIT_ARRAY))
        .phase("fini array")
        .register(SysvInitArray, section(SHT_FINI_ARRAY))
        .phase("start")
        .register(SysvStart, ObjectFilter::any())
}

// ————————————————————————————————— Phases ————————————————————————————————— //

pub struct PhaseHandle {
    fold: Fold,
    phase: usize,
}

impl PhaseHandle {
    pub fn register<I>(mut self, module: impl Module + 'static, item: I) -> Fold
    where
        I: Into<ItemFilter>,
    {
        let phase = &mut self.fold.phases[self.phase];

        let id = item.into();
        let mod_idx = phase.modules.push(Box::new(module));
        phase.filters.push((id, mod_idx));

        self.fold
    }

    pub fn delete(mut self) -> Fold {
        self.fold.phases.remove(self.phase);
        self.fold
    }

    pub fn after(self) -> Self {
        PhaseHandle {
            fold: self.fold,
            phase: self.phase + 1,
        }
    }

    pub fn create(mut self, name: impl AsRef<str>) -> Self {
        self.fold.phases.insert(
            self.phase,
            Phase {
                name: name.as_ref().to_string(),
                modules: Arena::new(),
                filters: Vec::new(),
            },
        );

        self
    }
}

impl Fold {
    /// Declare a new phase.
    pub fn phase(mut self, name: impl AsRef<str>) -> Self {
        self.phases.push(Phase {
            name: name.as_ref().to_string(),
            modules: Arena::new(),
            filters: Vec::new(),
        });
        self
    }

    pub fn select(self, name: impl AsRef<str>) -> PhaseHandle {
        if let Some(index) = self.phases.iter().position(|p| p.name == name.as_ref()) {
            PhaseHandle {
                fold: self,
                phase: index,
            }
        } else {
            panic!("Unable to find phase \"{}\"", name.as_ref());
        }
    }

    pub fn apply<F: FnMut(PhaseHandle) -> Self>(self, name: impl AsRef<str>, mut map: F) -> Self {
        map(self.select(name))
    }

    // Insert a phase after another
    pub fn insert_phase_after(mut self, name: impl AsRef<str>, other: impl AsRef<str>) -> Self {
        if let Some(index) = self.phases.iter().position(|p| p.name == other.as_ref()) {
            self.phases.insert(
                index + 1,
                Phase {
                    name: name.as_ref().to_string(),
                    modules: Arena::new(),
                    filters: Vec::new(),
                },
            );
        } else {
            log::warn!("Adding phase '{}' doesn't exists, ignoring", other.as_ref());
        }

        self
    }

    /// Register a module for the current phase.
    pub fn register_in_phase<I>(
        mut self,
        phase: impl AsRef<str>,
        module: impl Module + 'static,
        item: I,
    ) -> Self
    where
        I: Into<ItemFilter>,
    {
        let Some(phase) = self.phases.iter_mut().find(|p| p.name == phase.as_ref()) else {
            log::warn!(
                "Adding module '{}' but no phase declared yet, ignoring",
                module.name()
            );
            return self;
        };

        let id = item.into();
        let mod_idx = phase.modules.push(Box::new(module));
        phase.filters.push((id, mod_idx));
        self
    }

    // Insert a phase after another
    pub fn push_front_phase(mut self, name: impl AsRef<str>) -> Self {
        self.phases.insert(
            0,
            Phase {
                name: name.as_ref().to_string(),
                modules: Arena::new(),
                filters: Vec::new(),
            },
        );
        self
    }

    /// Register a module for the current phase.
    pub fn register<I>(mut self, module: impl Module + 'static, item: I) -> Self
    where
        I: Into<ItemFilter>,
    {
        let Some(phase) = self.phases.last_mut() else {
            log::warn!(
                "Adding module '{}' but no phase declared yet, ignoring",
                module.name()
            );
            return self;
        };

        let id = item.into();
        let mod_idx = phase.modules.push(Box::new(module));
        phase.filters.push((id, mod_idx));
        self
    }

    pub fn search_path(mut self, path: impl AsRef<str>) -> Self {
        self.search_path.push(path.as_ref().to_string());
        self
    }

    pub fn run(mut self) {
        let mut manifold = Manifold::new(self.config.env);

        // Load target
        let target = self.config.target;
        log::info!("Target: {target:?}");
        let file_fd = file::open_file_ro(target.to_bytes()).expect("Target is not a file");
        let file = file::map_file(file_fd);
        manifold.add_elf_file(file, target.to_owned());
        manifold
            .shared
            .insert(SYSV_COLLECTOR_SEARCH_PATHS_KEY, self.search_path);

        // Execute each phase
        for phase in &mut self.phases {
            log::info!("[ Phase: {} ]", phase.name);
            Self::drive_phase(phase, &mut manifold);
        }
    }

    /// Applies the modules of the phase to every objects.
    fn drive_phase(phase: &mut Phase, manifold: &mut Manifold) {
        for (filter, idx) in &phase.filters {
            if matches!(filter, ItemFilter::ManifoldFilter) {
                let module: &mut Box<dyn Module> = &mut phase.modules[*idx];
                module.process_manifold(manifold).unwrap();
            }
        }

        for handle in manifold.objects.handle_generator() {
            if manifold.objects.get(handle).is_none() {
                // We processed all the objects
                break;
            }

            Self::apply_modules(handle, phase, manifold);
        }
    }

    /// Applies all modules to an object.
    fn apply_modules(obj: Handle<Object>, phase: &mut Phase, manifold: &mut Manifold) {
        for (filter, idx) in &phase.filters {
            let module: &mut Box<dyn Module> = &mut phase.modules[*idx];

            if !filter
                .object_filter()
                .is_some_and(|f| manifold[obj].matches(f))
            {
                // Object does not match
                continue;
            }

            match filter {
                ItemFilter::ManifoldFilter => {}
                ItemFilter::Object(_) => {
                    if let Err(err) = module.process_object(obj, manifold) {
                        log::error!(
                            "Unable to process object {:?} with module {}: {err:#?}",
                            manifold.objects.get(obj).map(|o| o.display_path()),
                            module.name()
                        );
                        panic!();
                    }
                }
                ItemFilter::Segment(segment, _) => {
                    // *cries in functional programming, but borrow checker is angry*
                    let mut idx = 0;
                    while let Some(handle) = manifold[obj].segments.get(idx) {
                        idx += 1;
                        if manifold[*handle].tag == segment.tag {
                            if let Err(err) = module.process_segment(*handle, manifold) {
                                log::error!(
                                    "Unable to process segment #{} of object {:?} with module {}: {err:#?}",
                                    idx,
                                    manifold.objects.get(obj).map(|o| o.display_path()),
                                    module.name()
                                );
                                panic!();
                            }
                        }
                    }
                }
                ItemFilter::Section(section, _) => {
                    let mut idx = 0;
                    while let Some(handle) = manifold[obj].sections.get(idx) {
                        idx += 1;
                        if manifold[*handle].tag == section.tag {
                            if let Err(err) = module.process_section(*handle, manifold) {
                                log::error!(
                                    "Unable to process section #{} of object {:?} with module {}: {err:#?}",
                                    idx,
                                    manifold.objects.get(obj).map(|o| o.display_path()),
                                    module.name()
                                );
                                panic!();
                            }
                        }
                    }
                }
            }
        }
    }
}
