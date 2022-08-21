use prost_build::{Service, ServiceGenerator};

use crate::config::GrpcConfig;

pub(crate) struct PoemServiceGenerator {
    pub(crate) config: GrpcConfig,
}

impl ServiceGenerator for PoemServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        if self.config.build_client {
            crate::client::generate(&self.config, &service, buf);
        }
        if self.config.build_server {
            crate::server::generate(&self.config, &service, buf);
        }
    }
}
