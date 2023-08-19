use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::arena::Arena;
use crate::cli;
use crate::cli::Config;
use crate::env::Env;
use crate::manifold::{ItemFilter, Manifold};
use crate::module::CollectHandler;

use rustix::path::Arg;

// —————————————————————————————— Fold Driver ——————————————————————————————— //

pub struct Fold<T> {
    /// Inner state, depending on the phase
    s: T,
}

pub fn new(env: Env) -> Fold<Init> {
    log::info!("Hello, world!");
    log::info!("Args: {:?}", &env.args);

    let config = cli::parse(env);

    Fold { s: Init { config } }
}

// ————————————————————————————————— Phases ————————————————————————————————— //

pub struct Init {
    config: Config,
}

impl Fold<Init> {
    pub fn collect(self) -> Fold<Collect> {
        Fold {
            s: Collect {
                search_path: Vec::new(),
                config: self.s.config,
                collect: Arena::new(),
            },
        }
    }
}

pub struct Collect {
    config: Config,
    search_path: Vec<String>,
    collect: Arena<Box<dyn CollectHandler>>,
}

impl Fold<Collect> {
    pub fn register<I>(mut self, module: impl CollectHandler + 'static, item: I) -> Self
    where
        I: Into<ItemFilter>,
    {
        let id = item.into();
        log::info!("Collect {:?} with '{}'", id, module.name());
        self.s.collect.push(Box::new(module));
        self
    }

    pub fn search_path(mut self, path: impl AsRef<str>) -> Self {
        self.s.search_path.push(path.as_ref().to_string());
        self
    }

    pub fn build(self) -> Fold<Ready> {
        let s = self.s;
        Fold {
            s: Ready {
                config: s.config,
                search_path: s.search_path,
                manifold: Manifold::new(),
                collect: s.collect,
            },
        }
    }
}

pub struct Ready {
    config: Config,
    manifold: Manifold,
    search_path: Vec<String>,
    collect: Arena<Box<dyn CollectHandler>>,
}

impl Fold<Ready> {
    pub fn load(mut self) {
        let s = &self.s;

        self.collect();
    }

    fn collect(&mut self) {
        log::info!("Phase: collect");

        let s = &mut self.s;
        let target = s.config.target;
        log::info!("Target: {:?}", target);
    }
}
