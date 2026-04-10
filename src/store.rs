use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

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
    pub preview: String,
    pub stats: String,
    pub time_str: String,   // pre-formatted "HH:MM" or "MM/DD HH:MM"
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
        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        Self { id, content, time: now, pinned: false, preview, stats, time_str, text_lc }
    }

    /// Rebuild time_str after loading from disk (date may differ from today)
    pub fn rebuild_time_str(&mut self, today: chrono::NaiveDate) {
        let date = self.time.date_naive();
        self.time_str = if date == today {
            self.time.format("%H:%M").to_string()
        } else {
            self.time.format("%m/%d %H:%M").to_string()
        };
    }
}

fn make_preview(content: &ClipContent) -> String {
    match content {
        ClipContent::Text(s) => {
            let s = s.trim();
            let end = s.char_indices().nth(120).map(|(i, _)| i).unwrap_or(s.len());
            s[..end].to_string()
        }
        ClipContent::Image { .. } => "[图片]".to_string(),
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
    dirty: bool,
    last_save: Instant,
    /// HashSet of text content for O(1) dedup check
    text_set: std::collections::HashSet<String>,
}

impl Store {
    pub fn load() -> Self {
        let path = data_path();
        let mut entries: Vec<ClipEntry> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        entries.retain(|e| matches!(&e.content, ClipContent::Text(_)));
        let today = Local::now().date_naive();
        for e in &mut entries {
            if let ClipContent::Text(t) = &e.content {
                e.text_lc = t.to_lowercase();
            }
            e.rebuild_time_str(today);
        }
        let text_set: std::collections::HashSet<String> = entries.iter()
            .filter_map(|e| if let ClipContent::Text(t) = &e.content { Some(t.clone()) } else { None })
            .collect();
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self { entries, path, next_id, dirty: false, last_save: Instant::now(), text_set }
    }

    pub fn push(&mut self, content: ClipContent) {
        // O(1) check, then targeted remove by id instead of full retain scan
        if let ClipContent::Text(ref t) = content {
            if self.text_set.contains(t.as_str()) {
                if let Some(pos) = self.entries.iter().position(|e| match &e.content {
                    ClipContent::Text(existing) => existing == t,
                    _ => false,
                }) {
                    self.entries.remove(pos);
                }
                self.text_set.remove(t.as_str());
            }
        }
        let entry = ClipEntry::new(self.next_id, content);
        self.next_id += 1;
        if let ClipContent::Text(ref t) = entry.content {
            self.text_set.insert(t.clone());
        }
        // Insert after pinned entries
        let first_unpinned = self.entries.iter().position(|e| !e.pinned).unwrap_or(self.entries.len());
        self.entries.insert(first_unpinned, entry);
        // Trim unpinned to 500, also remove from text_set
        let mut unpinned = 0usize;
        self.entries.retain(|e| {
            if e.pinned { return true; }
            unpinned += 1;
            if unpinned > 500 {
                if let ClipContent::Text(t) = &e.content { self.text_set.remove(t.as_str()); }
                return false;
            }
            true
        });
        self.mark_dirty();
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(e) = self.entries.iter().find(|e| e.id == id) {
            if let ClipContent::Text(t) = &e.content { self.text_set.remove(t.as_str()); }
        }
        self.entries.retain(|e| e.id != id);
        self.mark_dirty();
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.pinned = !e.pinned;
        }
        self.mark_dirty();
    }

    pub fn clear_unpinned(&mut self) {
        self.entries.retain(|e| {
            if e.pinned { return true; }
            if let ClipContent::Text(t) = &e.content { self.text_set.remove(t.as_str()); }
            false
        });
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Call once per frame — flushes to disk if dirty and 2s have elapsed
    pub fn flush_if_needed(&mut self) {
        if !self.dirty { return; }
        if self.last_save.elapsed().as_secs() < 2 { return; }
        self.flush_now();
    }

    /// Force immediate save (call on app exit)
    pub fn flush_now(&mut self) {
        if !self.dirty { return; }
        let path = &self.path;
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        // Write directly to file via writer — avoids intermediate String allocation
        if let Ok(file) = std::fs::File::create(path) {
            let text_only = self.entries.iter()
                .filter(|e| matches!(e.content, ClipContent::Text(_)));
            // Collect refs for serde — serde_json::to_writer needs a slice
            let refs: Vec<&ClipEntry> = text_only.collect();
            let _ = serde_json::to_writer(std::io::BufWriter::new(file), &refs);
        }
        self.dirty = false;
        self.last_save = Instant::now();
    }
}

fn data_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("clip-vault")
        .join("history.json")
}
