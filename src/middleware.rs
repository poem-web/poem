use crate::Endpoint;

pub trait Middleware {
    fn transform<T: Endpoint>(&self, ep: T) -> Box<dyn Endpoint>;
}
