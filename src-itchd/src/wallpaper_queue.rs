// called wallpaper here instead of background to not be confused with the verb background.

use std::sync::Arc;

use anyhow::anyhow;
use tokio::{
    sync::{Mutex, mpsc},
    time::Duration,
};

mod builder;
mod scheduler;

pub use builder::WallpaperQueueBuilder;
use scheduler as sch;

#[derive(Clone)]
pub struct WallpaperQueue {
    pub queue: Arc<Mutex<Queue>>,
    pub scheduler: SchedulerRemote,
    pub current_index: Arc<Mutex<usize>>,
}

pub struct Queue {
    v: Vec<String>,
}

struct Scheduler {
    queue: Arc<Mutex<Queue>>,
    command_rx: mpsc::Receiver<sch::Command>,
    interval: Duration,
    current_index: Arc<Mutex<usize>>,
}

#[derive(Clone)]
pub struct SchedulerRemote {
    command_tx: mpsc::Sender<sch::Command>,
}

impl WallpaperQueue {
    pub fn builder() -> WallpaperQueueBuilder {
        WallpaperQueueBuilder::new()
    }

    pub fn new(initial_queue: Vec<String>) -> Self {
        let queue = Arc::new(Mutex::new(Queue::new(Some(initial_queue))));
        let current_index = Arc::new(Mutex::new(0));

        Self {
            queue: queue.clone(),
            scheduler: Scheduler::start(queue, current_index.clone()),
            current_index,
        }
    }

    pub async fn switch_to_wallpaper(&self, bg: &str) -> anyhow::Result<()> {
        let lock = self.queue.lock().await;

        let bg_index = lock
            .v
            .iter()
            .position(|v| v.as_str() == bg)
            .ok_or(anyhow!("Background is not in queue"))?;

        drop(lock);

        self.scheduler
            .reset_timeout_and_set_index(bg_index)
            .await
            .expect("Scheduler should be available");
        Ok(())
    }
}

impl Queue {
    pub fn new(v: Option<Vec<String>>) -> Self {
        Self {
            v: v.unwrap_or_default(),
        }
    }
}
