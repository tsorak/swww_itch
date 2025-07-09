use std::path::Path;

use super::*;

#[derive(Debug)]
pub struct WallpaperQueueBuilder {
    initial_queue: Vec<String>,
}

impl WallpaperQueueBuilder {
    pub(super) fn new() -> Self {
        Self {
            initial_queue: vec![],
        }
    }

    pub async fn with_initial_queue_from_directory<P: AsRef<Path>>(mut self, directory: P) -> Self {
        let mut queue = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(directory).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(bg) = entry
                    .path()
                    .canonicalize()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .map_or(None, |path| match path.as_str().rsplit_once('.')?.1 {
                        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "avif"
                        | "tga" | "pnm" | "farbfeld" => Some(path),
                        _ => None,
                    })
                {
                    queue.push(bg);
                }
            }
        }
        self.initial_queue = queue;
        self
    }

    pub fn dbg(self) -> Self {
        dbg!(&self);
        self
    }

    pub fn build(self) -> WallpaperQueue {
        WallpaperQueue::new(self.initial_queue)
    }
}
