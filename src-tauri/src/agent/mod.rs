pub mod tools;

use crate::llm::ToolDefinition;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

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
    fn is_destructive(&self) -> bool {
        false
    }
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
            Arc::new(tools::ListSimilarTool::new(db.clone())),
            Arc::new(tools::MoveFileTool::new()),
            Arc::new(tools::TagFileTool::new(db)),
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

#[derive(Default)]
pub struct ConfirmationRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ConfirmationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, call_id: String) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(call_id, tx);
        rx
    }

    pub async fn resolve(&self, call_id: &str, approved: bool) -> bool {
        if let Some(tx) = self.pending.lock().await.remove(call_id) {
            let _ = tx.send(approved);
            true
        } else {
            false
        }
    }

    pub async fn cancel(&self, call_id: &str) {
        self.pending.lock().await.remove(call_id);
    }
}
