use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::arena::{Arena, Handle};
use crate::cli::Config;
use crate::env::Env;
use crate::filters::ItemFilter;
use crate::manifold::Manifold;
use crate::module::Module;
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

pub fn new(env: Env) -> Fold {
    log::info!("Hello, world!");
    log::info!("Args: {:?}", &env.args);

    let config = cli::parse(env);

    let path =
        &config.target.to_string_lossy()[..config.target.to_string_lossy().rfind('/').unwrap()];

    let search_path = Vec::from(&[path.to_owned()]);

    Fold {
        config,
        search_path,
        phases: Vec::new(),
    }
}

// ————————————————————————————————— Phases ————————————————————————————————— //

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
        let mut manifold = Manifold::new();

        // Load target
        let target = self.config.target;
        log::info!("Target: {:?}", target);
        let file_fd = file::open_file_ro(target.to_bytes()).expect("Target is not a file");
        let file = file::map_file(file_fd);
        manifold.add_elf_file(file, target.to_owned());
        manifold.add_search_paths(self.search_path);

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
