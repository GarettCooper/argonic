use crate::method::MethodHandler;

pub struct ServerBuilder {}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn method(self, method: String, handler: impl MethodHandler) -> Self {
        self
    }

    pub fn build(self) -> Server {
        Server {}
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Server {}
