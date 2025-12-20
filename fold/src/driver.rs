use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::ffi::CString;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str::FromStr;

use goblin::elf::program_header::PT_LOAD;
use goblin::elf::section_header::{SHT_DYNAMIC, SHT_RELA};

use crate::arena::Handle;
use crate::cli::Config;
use crate::env::Env;
use crate::filters::Filter;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::musl::MuslLocator;
use crate::object::Object;
use crate::sysv::collector::{
    SysvRemappingCollector, SYSV_COLLECTOR_REMAP_KEY, SYSV_COLLECTOR_SEARCH_PATHS_KEY,
};
use crate::sysv::loader::SysvLoader;
use crate::sysv::protect::SysvProtect;
use crate::sysv::relocation::SysvReloc;
use crate::sysv::start::SysvStart;
use crate::sysv::tls::allocation::TlsAllocator;
use crate::sysv::tls::collection::TlsCollector;
use crate::sysv::tls::relocation::TlsRelocator;
use crate::{cli, file, ShareMap};

type ModuleRef = Box<dyn Module>;

// —————————————————————————————— Fold Driver ——————————————————————————————— //

/// Module chain that can be applied to an ELF object file.
///
/// It consists of several [`Module`] that are applied successively to a [`Manifold`]. Each module is registered along
/// with a [`Filter`], selecting on which elements of the [`Manifold`] the module must be applied to.
///
/// [`Fold`] can be constructed with either [`Fold::new`], [`Fold::default_chain`] or [`chain`][crate::chain], then modules can be registered
/// with the object's methods. Modules can also be removed or modified, allowing to easily modify already existing
/// chain.
pub struct Fold {
    config: Config,
    initial_share_map: ShareMap,
    phases: Vec<Phase>,
}

struct Phase {
    name: String,
    module: ModuleRef,
    filter: Filter,
}

impl Fold {
    /// Creates an empty [`Fold`] from the execution context (`env`) and the name of the linker's binary (`linker_name`).
    ///
    /// `linker_name` is required in order to identify whether the linker was explicitely invoked (`/lib/linker exe`) or
    /// implicitely by the kernel (`./exe`).
    pub fn new(env: Env, linker_name: &str) -> Fold {
        log::info!("Hello, world!");
        log::info!("Args: {:?}", &env.args);

        let config = cli::parse(env, linker_name);

        Fold {
            config,
            initial_share_map: ShareMap::new(),
            phases: Vec::new(),
        }
    }

    /// Creates a [`Fold`] with a default chain of [`sysv`][crate::sysv] modules, able to link x86 executables
    ///
    /// See [`Fold::new`] for details on the arguments.
    pub fn default_chain(env: Env, linker_name: &str) -> Fold {
        let mut fold = Self::new(env, linker_name)
            .register(
                "collect",
                SysvRemappingCollector,
                Filter::section_type(SHT_DYNAMIC),
            )
            .register("load", SysvLoader, Filter::segment_type(PT_LOAD))
            .register("musl-locator", MuslLocator, Filter::manifold())
            .register("tls-collector", TlsCollector, Filter::any_object())
            .register("tls-allocator", TlsAllocator, Filter::manifold())
            .register(
                "tls-relocator",
                TlsRelocator,
                Filter::section_type(SHT_RELA),
            )
            .register(
                "relocation",
                SysvReloc::new(),
                Filter::any_object(), // TODO: match only elf
            )
            .register("protect", SysvProtect, Filter::segment_type(PT_LOAD))
            .register("start", SysvStart, Filter::any_object());

        // Compute the search paths for shared librairies.
        {
            let cwd = if let Some(last_delim) = fold.config.target.to_string_lossy().rfind('/') {
                &fold.config.target.to_string_lossy()[..last_delim]
            } else {
                "."
            };
            fold.initial_share_map.insert(
                SYSV_COLLECTOR_SEARCH_PATHS_KEY,
                [cwd, "musl/lib", "/lib", "/lib64", "/usr/lib/"]
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect(),
            );
        }

        // Compute libc remapping to use musl
        {
            fn cs(s: &str) -> CString {
                CString::from_str(s).unwrap()
            }

            // Replace the versionned libc (e.g. libc.so.6) present in the deps by musl's unversionned libc
            let mut map = BTreeMap::new();
            map.insert(
                "libc.so".to_owned(),
                Some(CString::from_str("libc.so").unwrap()),
            );

            let to_drop = [
                "ld-linux-x86-64.so",
                "libcrypt.so",
                "libdl.so",
                "libm.so",
                "libpthread.so",
                "libresolv.so",
                "librt.so",
                "libutil.so",
                "libxnet.so",
            ];

            for d in to_drop {
                map.insert(d.to_owned(), None);
            }

            fold.initial_share_map.insert(SYSV_COLLECTOR_REMAP_KEY, map);
        }

        fold
    }

    /// Creates a [`ModuleHandle`] to modify the module with named `name`.
    pub fn select(self, name: impl AsRef<str>) -> ModuleHandle {
        if let Some(index) = self.phases.iter().position(|p| p.name == name.as_ref()) {
            ModuleHandle {
                fold: self,
                phase: index,
            }
        } else {
            panic!("Unable to find module \"{}\"", name.as_ref());
        }
    }

    /// Identifies the module registered as `name` and update it using the provided mapping function, returning the
    /// output of the mapping function.
    pub fn apply<F: FnMut(ModuleHandle) -> R, R>(self, name: impl AsRef<str>, mut map: F) -> R {
        map(self.select(name))
    }

    /// Creates a [`PositionedModuleHandle`] at the start of the chain.
    pub fn front(self) -> PositionedModuleHandle {
        PositionedModuleHandle {
            hdx: ModuleHandle {
                fold: self,
                phase: 0,
            },
            position: CursorPosition::Before,
        }
    }

    /// Creates a [`PositionedModuleHandle`] at the end of the chain.
    pub fn back(self) -> PositionedModuleHandle {
        let phase = self.phases.len() - 1;
        PositionedModuleHandle {
            hdx: ModuleHandle { fold: self, phase },
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
            filter: item,
        });

        self
    }

    /// A mutable reference to the initial [`ShareMap`].
    ///
    /// This can be used to add initial values to the [`Manifold::shared`] map used during [`run`][Fold::run].
    pub fn share_map(&mut self) -> &mut ShareMap {
        &mut self.initial_share_map
    }

    /// Executes the [`Fold`] modules on a [`Manifold`] built from the execution context and target object file.
    pub fn run(mut self) {
        let mut manifold = Manifold::new(self.config.env, self.initial_share_map);

        // Load target
        let target = self.config.target;
        log::info!("Target: {target:?}");
        let file_fd = file::open_file_ro(target.to_bytes()).expect("Target is not a file");
        let file = file::map_file(file_fd);
        manifold.add_elf_file(file, target.to_owned());

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

/// Handle used to modify an already existing module in a [`Fold`].
///
/// It can replace it with another module or delete it. It may also be positioned before or after a module to insert new
///  ones.
pub struct ModuleHandle {
    fold: Fold,
    phase: usize,
}

impl ModuleHandle {
    /// Deletes the selected module, effectively removing it from the chain.
    pub fn delete(mut self) -> Fold {
        self.fold.phases.remove(self.phase);
        self.fold
    }

    /// Replaces the selected module with a new name, module and filter.
    pub fn replace(
        mut self,
        name: impl AsRef<str>,
        module: impl Module + 'static,
        item: Filter,
    ) -> Fold {
        let phase = &mut self.fold.phases[self.phase];

        phase.name = name.as_ref().to_owned();
        phase.module = Box::new(module);
        phase.filter = item;

        self.fold
    }

    /// Creates a [`PositionedModuleHandle`] handle between the selected one and the next one, allowing insertion of
    /// modules to be executed after the selected one.
    pub fn after(self) -> PositionedModuleHandle {
        PositionedModuleHandle {
            hdx: self,
            position: CursorPosition::After,
        }
    }

    /// Creates a [`PositionedModuleHandle`] handle between the selected one and the previous one, allowing insertion of
    /// modules to be executed before the selected one.
    pub fn before(self) -> PositionedModuleHandle {
        PositionedModuleHandle {
            hdx: self,
            position: CursorPosition::Before,
        }
    }
}

enum CursorPosition {
    Before,
    After,
}

/// Handle used to new modules relatively to other ones.
pub struct PositionedModuleHandle {
    hdx: ModuleHandle,
    position: CursorPosition,
}

impl PositionedModuleHandle {
    /// Returns the original [`ModuleHandle`].
    pub fn as_handle(self) -> ModuleHandle {
        self.hdx
    }

    /// Registers a new module at the position of the handle.
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
                filter: item,
            },
        );

        self.hdx.fold
    }
}
