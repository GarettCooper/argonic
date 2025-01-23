use std::future::Future;

use crate::response::Response;

pub trait MethodHandler {
    type Future: Future<Output = Response> + Send + 'static;

    fn call(&self) -> Self::Future;

    // TODO: Add a layer to the method
    // fn layer(&self) -> Self::Layer;
}

pub trait MethodService {}
