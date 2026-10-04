//! Types for resources.

use std::future::Future;

use crate::protocol::{
    resources::{
        Resource, ResourceTemplate, ResourcesListRequest, ResourcesListResponse,
        ResourcesReadRequest, ResourcesReadResponse, ResourcesTemplatesListRequest,
        ResourcesTemplatesListResponse,
    },
    rpc::RpcError,
};

/// Represents a resources collection.
pub trait Resources {
    /// Returns a list of resources.
    fn list(
        &self,
        request: ResourcesListRequest,
    ) -> impl Future<Output = Result<ResourcesListResponse, RpcError>> + Send;

    /// Returns a list of resource templates.
    fn templates(
        &self,
        request: ResourcesTemplatesListRequest,
    ) -> impl Future<Output = Result<ResourcesTemplatesListResponse, RpcError>> + Send;

    /// Reads a resource by uri.
    fn read(
        &self,
        request: ResourcesReadRequest,
    ) -> impl Future<Output = Result<ResourcesReadResponse, RpcError>> + Send;
}

/// Empty resources collection.
#[derive(Debug, Clone, Copy)]
pub struct NoResources;

impl Resources for NoResources {
    #[inline]
    async fn list(
        &self,
        _request: ResourcesListRequest,
    ) -> Result<ResourcesListResponse, RpcError> {
        Ok(ResourcesListResponse {
            resources: Vec::<Resource>::new(),
        })
    }

    #[inline]
    async fn templates(
        &self,
        _request: ResourcesTemplatesListRequest,
    ) -> Result<ResourcesTemplatesListResponse, RpcError> {
        Ok(ResourcesTemplatesListResponse {
            resource_templates: Vec::<ResourceTemplate>::new(),
        })
    }

    #[inline]
    async fn read(&self, request: ResourcesReadRequest) -> Result<ResourcesReadResponse, RpcError> {
        Err(RpcError::invalid_params(format!(
            "resource not found: {}",
            request.uri
        )))
    }
}
