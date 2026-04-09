use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub enum ClipContent {
    Text(String),
    #[serde(skip)]
    Image { width: u32, height: u32, rgba: Vec<u8> },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ClipEntry {
    pub id: u64,
    pub content: ClipContent,
    pub time: DateTime<Local>,
    pub pinned: bool,
    /// Pre-computed: first 120 chars trimmed
    pub preview: String,
    /// Pre-computed: "N chars" or "N×M image"
    pub stats: String,
    /// Pre-computed lowercase text for fast search
    #[serde(skip)]
    pub text_lc: String,
}

impl ClipEntry {
    pub fn new(id: u64, content: ClipContent) -> Self {
        let preview = make_preview(&content);
        let stats = make_stats(&content);
        let text_lc = match &content {
            ClipContent::Text(t) => t.to_lowercase(),
            ClipContent::Image { .. } => String::new(),
        };
        Self { id, content, time: Local::now(), pinned: false, preview, stats, text_lc }
    }
}

fn make_preview(content: &ClipContent) -> String {
    match content {
        ClipContent::Text(s) => {
            let s = s.trim();
            let end = s.char_indices().nth(120).map(|(i, _)| i).unwrap_or(s.len());
            s[..end].to_string()
        }
        ClipContent::Image { .. } => "[Image]".to_string(),
    }
}

fn make_stats(content: &ClipContent) -> String {
    match content {
        ClipContent::Text(s) => {
            let chars = s.chars().count();
            let lines = s.lines().count();
            if lines > 1 { format!("{} 行 {} 字", lines, chars) }
            else { format!("{} 字", chars) }
        }
        ClipContent::Image { width, height, .. } => format!("{}×{}", width, height),
    }
}

pub struct Store {
    pub entries: Vec<ClipEntry>,
    path: PathBuf,
    next_id: u64,
}

impl Store {
    pub fn load() -> Self {
        let path = data_path();
        let mut entries: Vec<ClipEntry> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        entries.retain(|e| matches!(&e.content, ClipContent::Text(_)));
        // Rebuild text_lc (skipped during deserialization)
        for e in &mut entries {
            if let ClipContent::Text(t) = &e.content {
                e.text_lc = t.to_lowercase();
            }
        }
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self { entries, path, next_id }
    }

    pub fn push(&mut self, content: ClipContent) {
        if let ClipContent::Text(ref new_text) = content {
            self.entries.retain(|e| match &e.content {
                ClipContent::Text(t) => t != new_text,
                _ => true,
            });
        }

        let entry = ClipEntry::new(self.next_id, content);
        self.next_id += 1;

        let first_unpinned = self.entries.iter().position(|e| !e.pinned).unwrap_or(self.entries.len());
        self.entries.insert(first_unpinned, entry);

        let mut unpinned = 0usize;
        self.entries.retain(|e| {
            if e.pinned { return true; }
            unpinned += 1;
            unpinned <= 500
        });

        self.save_async();
    }

    pub fn remove(&mut self, id: u64) {
        self.entries.retain(|e| e.id != id);
        self.save_async();
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.pinned = !e.pinned;
        }
        self.save_async();
    }

    pub fn clear_unpinned(&mut self) {
        self.entries.retain(|e| e.pinned);
        self.save_async();
    }

    fn save_async(&self) {
        let text_only: Vec<&ClipEntry> = self.entries.iter()
            .filter(|e| matches!(e.content, ClipContent::Text(_)))
            .collect();
        if let Ok(json) = serde_json::to_string(&text_only) {
            let path = self.path.clone();
            std::thread::spawn(move || {
                let _ = std::fs::create_dir_all(path.parent().unwrap());
                let _ = std::fs::write(&path, json);
            });
        }
    }
}

fn data_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("clip-vault")
        .join("history.json")
}
