use std::sync::Arc;

use http::StatusCode;
use poem::{FromRequest, Request, RequestBody};
use serde::de::DeserializeOwned;
use worker::{
    Ai, AnalyticsEngineDataset, Bucket, DynamicDispatcher, EnvBinding, Fetcher, Hyperdrive,
    ObjectNamespace, Secret, Var, kv::KvStore,
};

#[derive(Clone)]
pub struct Env(pub(crate) Arc<worker::Env>);

impl Env {
    pub fn new(env: worker::Env) -> Self {
        Self(Arc::new(env))
    }

    pub fn get_binding<T: EnvBinding>(&self, name: &str) -> worker::Result<T> {
        self.0.get_binding(name)
    }

    pub fn ai(&self, binding: &str) -> worker::Result<Ai> {
        self.0.ai(binding)
    }

    pub fn analytics_engine(&self, binding: &str) -> worker::Result<AnalyticsEngineDataset> {
        self.0.analytics_engine(binding)
    }

    pub fn secret(&self, binding: &str) -> worker::Result<Secret> {
        self.0.secret(binding)
    }

    pub fn var(&self, binding: &str) -> worker::Result<Var> {
        self.0.var(binding)
    }

    pub fn object_var<T: DeserializeOwned>(&self, binding: &str) -> worker::Result<T> {
        self.0.object_var(binding)
    }

    pub fn kv(&self, binding: &str) -> worker::Result<KvStore> {
        self.0.kv(binding)
    }

    pub fn durable_object(&self, binding: &str) -> worker::Result<ObjectNamespace> {
        self.0.durable_object(binding)
    }

    pub fn dynamic_dispatcher(&self, binding: &str) -> worker::Result<DynamicDispatcher> {
        self.0.dynamic_dispatcher(binding)
    }

    pub fn service(&self, binding: &str) -> worker::Result<Fetcher> {
        self.0.service(binding)
    }

    #[cfg(feature = "queue")]
    pub fn queue(&self, binding: &str) -> worker::Result<worker::Queue> {
        self.0.queue(binding)
    }

    pub fn bucket(&self, binding: &str) -> worker::Result<Bucket> {
        self.0.bucket(binding)
    }

    #[cfg(feature = "d1")]
    pub fn d1(&self, binding: &str) -> worker::Result<worker::D1Database> {
        self.0.d1(binding)
    }

    pub fn assets(&self, binding: &str) -> worker::Result<Fetcher> {
        self.0.assets(binding)
    }

    pub fn hyperdrive(&self, binding: &str) -> worker::Result<Hyperdrive> {
        self.0.hyperdrive(binding)
    }
}

impl<'a> FromRequest<'a> for Env {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let env = req.data::<Env>().ok_or_else(|| {
            poem::Error::from_string("failed to get incoming env", StatusCode::BAD_REQUEST)
        })?;

        Ok(env.clone())
    }
}
