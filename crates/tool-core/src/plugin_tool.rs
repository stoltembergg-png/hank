use crate::{PolicyDecision, Tool, ToolError, ToolRequest, ToolResponse, ToolSchema};
use async_trait::async_trait;
use std::sync::Arc;

const MAX_IDENTITY: usize = 256;

pub struct ToolPluginAdapter<T: Tool + ?Sized> {
    inner: Arc<T>,
    plugin_id: String,
    plugin_digest: String,
    approved: bool,
}

impl<T: Tool + ?Sized> ToolPluginAdapter<T> {
    pub fn new(
        inner: Arc<T>,
        plugin_id: &str,
        plugin_digest: &str,
        approved: bool,
    ) -> Result<Self, ToolError> {
        if plugin_id.is_empty()
            || plugin_digest.is_empty()
            || plugin_id.len() > MAX_IDENTITY
            || plugin_digest.len() > MAX_IDENTITY
        {
            return Err(ToolError::Internal("invalid plugin identity".into()));
        }
        Ok(Self {
            inner,
            plugin_id: plugin_id.into(),
            plugin_digest: plugin_digest.into(),
            approved,
        })
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn plugin_digest(&self) -> &str {
        &self.plugin_digest
    }

    fn require_approval(&self) -> Result<(), ToolError> {
        if self.approved {
            Ok(())
        } else {
            Err(ToolError::PermissionDenied {
                decision: PolicyDecision::Deny,
            })
        }
    }
}

#[async_trait]
impl<T: Tool + ?Sized> Tool for ToolPluginAdapter<T> {
    fn schema(&self) -> &'static ToolSchema {
        self.inner.schema()
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        self.require_approval()?;
        self.inner.can_handle(&request)?;
        self.inner.execute(request).await
    }

    fn can_handle(&self, request: &ToolRequest) -> Result<(), ToolError> {
        self.require_approval()?;
        self.inner.can_handle(request)
    }
}
