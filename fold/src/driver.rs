use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use goblin::elf::program_header::PT_LOAD;

use crate::arena::Handle;
use crate::cli::Config;
use crate::env::Env;
use crate::filters::Filter;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::sysv::collector::{SysvRemappingCollector, SYSV_COLLECTOR_SEARCH_PATHS_KEY};
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
    module: ModuleRef,
    filter: Filter,
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
        .search_paths(["/lib", "/lib64", "/usr/lib/"].iter())
        .register(
            "collect",
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
            Filter::any_object(),
        )
        .register("load", SysvLoader, Filter::segment_type(PT_LOAD))
        .register("tls", SysvTls, Filter::manifold())
        .register(
            "relocation",
            SysvReloc::new(),
            Filter::any_object(), // TODO: match only elf
        )
        .register("protect", SysvProtect, Filter::segment_type(PT_LOAD))
        .register("start", SysvStart, Filter::any_object())
}

// ————————————————————————————————— Phases ————————————————————————————————— //

impl Fold {
    /// Creates a `PhaseHandle` to modify the phase with named `name`.
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

    /// Identifies the phase `name` and update it using the provided mapping function, returning the output of the mapping
    /// function.
    pub fn apply<F: FnMut(PhaseHandle) -> R, R>(self, name: impl AsRef<str>, mut map: F) -> R {
        map(self.select(name))
    }

    /// Creates a `PositionedPhaseHandle` at the start of the chain.
    pub fn front(self) -> PositionedPhaseHandle {
        PositionedPhaseHandle {
            hdx: PhaseHandle {
                fold: self,
                phase: 0,
            },
            position: CursorPosition::Before,
        }
    }

    /// Creates a `PositionedPhaseHandle` at the end of the chain.
    pub fn back(self) -> PositionedPhaseHandle {
        let phase = self.phases.len() - 1;
        PositionedPhaseHandle {
            hdx: PhaseHandle { fold: self, phase },
            position: CursorPosition::After,
        }
    }

    /// Registers a module at the end of the chain.
    pub fn register(
        mut self,
        name: impl AsRef<str>,
        module: impl Module + 'static,
        item: Filter,
    ) -> Self {
        self.phases.push(Phase {
            name: name.as_ref().to_owned(),
            module: Box::new(module),
            filter: item.into(),
        });

        self
    }

    pub fn search_path(mut self, path: impl AsRef<str>) -> Self {
        self.search_path.push(path.as_ref().to_string());
        self
    }

    pub fn search_paths<I, S>(mut self, paths: I) -> Self
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
    {
        self.search_path
            .extend(paths.map(|s| s.as_ref().to_owned()));
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
        if phase.filter.matches_manifold() {
            let module: &mut Box<dyn Module> = &mut phase.module;
            module.process_manifold(manifold).unwrap();
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
        let module: &mut Box<dyn Module> = &mut phase.module;

        if phase.filter.matches_object(&manifold[obj]) {
            if let Err(err) = module.process_object(obj, manifold) {
                log::error!(
                    "Unable to process object {:?} with module {}: {err:#?}",
                    manifold.objects.get(obj).map(|o| o.display_path()),
                    module.name()
                );
                panic!();
            }
        }

        if phase.filter.is_segment_filter() {
            let mut idx = 0;
            while let Some(handle) = manifold[obj].segments.get(idx) {
                idx += 1;
                if phase
                    .filter
                    .matches_segment(&manifold[*handle], &manifold[obj])
                {
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

        if phase.filter.is_section_filter() {
            let mut idx = 0;
            while let Some(handle) = manifold[obj].sections.get(idx) {
                idx += 1;
                if phase
                    .filter
                    .matches_section(&manifold[*handle], &manifold[obj])
                {
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

/// Handle used to modify an already existing phase by either replacing it with another module or deleting it. It can
/// also be positioned before or after the phase to insert new ones.
pub struct PhaseHandle {
    fold: Fold,
    phase: usize,
}

impl PhaseHandle {
    /// Deletes the selected phase, effectively removing it from the chain.
    pub fn delete(mut self) -> Fold {
        self.fold.phases.remove(self.phase);
        self.fold
    }

    /// Replaces the selected phase with a new name, module and filter.
    pub fn replace(
        mut self,
        name: impl AsRef<str>,
        module: impl Module + 'static,
        item: Filter,
    ) -> Fold {
        let phase = &mut self.fold.phases[self.phase];

        phase.name = name.as_ref().to_owned();
        phase.module = Box::new(module);
        phase.filter = item.into();

        self.fold
    }

    /// Creates a `PositionedPhaseHandle` handle between the selected one and the next one, allowing insertion of
    /// phases to be executed after the selected one.
    pub fn after(self) -> PositionedPhaseHandle {
        PositionedPhaseHandle {
            hdx: self,
            position: CursorPosition::After,
        }
    }

    /// Creates a `PositionedPhaseHandle` handle between the selected one and the previous one, allowing insertion of
    /// phases to be executed before the selected one.
    pub fn before(self) -> PositionedPhaseHandle {
        PositionedPhaseHandle {
            hdx: self,
            position: CursorPosition::Before,
        }
    }
}

enum CursorPosition {
    Before,
    After,
}

/// Handle used to new phases relatively to other ones.
pub struct PositionedPhaseHandle {
    hdx: PhaseHandle,
    position: CursorPosition,
}

impl PositionedPhaseHandle {
    /// Returns the original `PhaseHandle`.
    pub fn as_handle(self) -> PhaseHandle {
        self.hdx
    }

    /// Registers a new phase at the position of the handle.
    pub fn register(
        mut self,
        name: impl AsRef<str>,
        module: impl Module + 'static,
        item: Filter,
    ) -> Fold {
        self.hdx.fold.phases.insert(
            self.hdx.phase
                + match self.position {
                    CursorPosition::After => 1,
                    CursorPosition::Before => 0,
                },
            Phase {
                name: name.as_ref().to_string(),
                module: Box::new(module),
                filter: item.into(),
            },
        );

        self.hdx.fold
    }
}
