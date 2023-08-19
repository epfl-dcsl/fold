pub trait Module {
    fn name(&self) -> &'static str;
}

pub trait CollectHandler: Module {}
