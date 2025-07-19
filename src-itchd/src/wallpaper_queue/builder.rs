use std::path::Path;
use tokio::time::Instant;

use super::*;

#[derive(Debug)]
pub struct WallpaperQueueBuilder {
    pub initial_queue: Vec<String>,
    pub db: Option<Sqlite>,
}

impl WallpaperQueueBuilder {
    pub(super) fn new() -> Self {
        Self {
            initial_queue: vec![],
            db: None,
        }
    }

    /// Loads the queue which has been sorted by the user.
    ///
    /// This should likely always be run before "Self::with_initial_queue_from_directory"
    /// as this one is user sorted.
    pub async fn with_ordered_queue(mut self) -> Self {
        println!("Loading user ordered queue...");
        let start = Instant::now();

        let db = self.db.unwrap_or(open_or_make_db().await.unwrap());

        let queue = db.read_queue().await;

        self.db = Some(db);

        self.push_unique_onto_queue(queue);

        println!(
            "Loaded user ordered queue. Took: {}µs",
            start.elapsed().as_micros()
        );

        self
    }

    pub async fn with_initial_queue_from_directory<P: AsRef<Path>>(mut self, directory: P) -> Self {
        println!("Loading background directory...");
        let start = Instant::now();

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
        self.push_unique_onto_queue(queue);

        println!(
            "Loaded background directory. Took: {}µs",
            start.elapsed().as_micros()
        );

        self
    }

    // Future todo: Multiple background directories.
    // Problems: having backgrounds with the same name in multiple directories
    // will(/should?) only use the first one encountered.
    // The Tauri app uses only filenames, not entire absolute filepaths backgrounds.
    //
    /// Will only push items which are not already in the queue
    pub fn push_unique_onto_queue(&mut self, vec: Vec<String>) {
        for new in vec.into_iter() {
            if !self.initial_queue.contains(&new) {
                self.initial_queue.push(new);
            }
        }
    }

    pub fn dbg(self) -> Self {
        dbg!(&self);
        self
    }

    pub fn dbg_queue(self) -> Self {
        dbg!(&self.initial_queue);
        self
    }

    pub async fn build(self) -> WallpaperQueue {
        WallpaperQueue::new(self.initial_queue, self.db).await
    }
}
