pub mod history;
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
            Arc::new(tools::MoveFileTool::new(db.clone())),
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

    fn make_key(conversation_id: i64, call_id: &str) -> String {
        format!("{}:{}", conversation_id, call_id)
    }

    pub async fn register(
        &self,
        conversation_id: i64,
        call_id: String,
    ) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        let key = Self::make_key(conversation_id, &call_id);
        let mut guard = self.pending.lock().await;
        if let Some(old) = guard.insert(key, tx) {
            let _ = old.send(false);
        }
        rx
    }

    pub async fn resolve(
        &self,
        conversation_id: i64,
        call_id: &str,
        approved: bool,
    ) -> bool {
        let key = Self::make_key(conversation_id, call_id);
        if let Some(tx) = self.pending.lock().await.remove(&key) {
            let _ = tx.send(approved);
            true
        } else {
            false
        }
    }

    pub async fn cancel_conversation(&self, conversation_id: i64) -> usize {
        let prefix = format!("{}:", conversation_id);
        let mut guard = self.pending.lock().await;
        let keys: Vec<String> = guard
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        let n = keys.len();
        for k in keys {
            if let Some(tx) = guard.remove(&k) {
                let _ = tx.send(false);
            }
        }
        n
    }
}
