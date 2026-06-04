pub mod tools;

use crate::llm::ToolDefinition;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
    pub result: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, args: &str) -> Result<String>;
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(tools::SearchFilesTool::new(db.clone())),
            Arc::new(tools::ReadFileTool::new()),
            Arc::new(tools::ListSimilarTool::new(db)),
        ];
        Self { tools }
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.definition().name == name).cloned()
    }
}
