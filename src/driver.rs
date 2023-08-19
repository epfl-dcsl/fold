use alloc::boxed::Box;

use crate::arena::Arena;
use crate::cli;
use crate::env::Env;
use crate::manifold::ItemFilter;
use crate::module::CollectHandler;

// —————————————————————————————— Fold Driver ——————————————————————————————— //

pub struct Fold<T> {
    /// Inner state, depending on the phase
    s: T,
}

pub fn new(env: Env) -> Fold<Init> {
    log::info!("Hello, world!");
    log::info!("Args: {:?}", &env.args);

    let _config = cli::parse(&env.args);

    Fold { s: Init {} }
}

// ————————————————————————————————— Phases ————————————————————————————————— //

pub struct Init {}

impl Fold<Init> {
    pub fn collect(self) -> Fold<Collect> {
        Fold {
            s: Collect {
                collect: Arena::new(),
            },
        }
    }
}

pub struct Collect {
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
}
