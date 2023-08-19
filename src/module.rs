use crate::manifold::Manifold;

pub trait Module {
    fn name(&self) -> &'static str;
}

pub trait CollectHandler: Module {
    fn collect(&mut self, manifodl: &mut Manifold);
}
