//! Provider port e adapters - stub inicial

pub struct ModelCapabilities {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub max_tokens: u32,
    pub max_context: u32,
}

pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> ModelCapabilities;
}

pub struct MockProvider {
    pub name: String,
}

impl ModelProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            streaming: true,
            tools: true,
            vision: false,
            max_tokens: 8192,
            max_context: 32768,
        }
    }
}
