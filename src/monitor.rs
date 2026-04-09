use crate::store::ClipContent;
use arboard::Clipboard;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn start(tx: Sender<ClipContent>) {
    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut last_text = String::new();
        let mut last_img_hash: u64 = 0;

        loop {
            // 200ms gives snappier feel vs 500ms, still low CPU
            thread::sleep(Duration::from_millis(200));

            if let Ok(text) = clipboard.get_text() {
                let text = text.trim().to_string();
                if !text.is_empty() && text != last_text {
                    last_text = text.clone();
                    let _ = tx.send(ClipContent::Text(text));
                }
            }

            if let Ok(img) = clipboard.get_image() {
                // Hash before copying bytes — skip copy if unchanged
                let hash = fnv1a(&img.bytes);
                if hash != last_img_hash {
                    last_img_hash = hash;
                    let _ = tx.send(ClipContent::Image {
                        width: img.width as u32,
                        height: img.height as u32,
                        rgba: img.bytes.into_owned(),
                    });
                }
            }
        }
    });
}

/// Fast non-cryptographic hash for change detection
fn fnv1a(data: &[u8]) -> u64 {
    let step = (data.len() / 512).max(1);
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data.iter().step_by(step) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
