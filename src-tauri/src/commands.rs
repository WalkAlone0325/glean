#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn get_stats() -> Stats {
    Stats {
        files: 0,
        chunks: 0,
        tags: 0,
    }
}

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}
