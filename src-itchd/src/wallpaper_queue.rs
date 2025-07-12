// called wallpaper here instead of background to not be confused with the verb background.

use std::sync::Arc;

use anyhow::anyhow;
use swww_itch_shared::message::Position;
use tokio::{
    sync::{Mutex, mpsc},
    time::Duration,
};

mod builder;
mod iter;
mod scheduler;

pub use builder::WallpaperQueueBuilder;
use iter::BackgroundIterable;
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

    pub async fn get_queue(&self) -> Vec<String> {
        self.queue.lock().await.to_vec()
    }

    pub async fn switch_to_wallpaper(&self, bg: &str) -> anyhow::Result<()> {
        let lock = self.queue.lock().await;

        let bg_index = lock
            .get_index_of_bg(bg)
            .ok_or(anyhow!("Background is not in queue"))?;

        drop(lock);

        self.scheduler
            .reset_timeout_and_set_index(bg_index)
            .await
            .expect("Scheduler should be available");
        Ok(())
    }

    pub async fn rearrange_wallpaper(
        &self,
        bg: &str,
        before_or_after: &Position,
        target_bg: &str,
    ) -> anyhow::Result<(usize, usize)> {
        let i_lock = self.current_index.lock().await;
        let mut lock = self.queue.lock().await;

        let queued_bg = lock
            .get_by_index(*i_lock)
            .expect("current_index should always point to an existing item")
            .to_owned();
        drop(i_lock);

        let bg_index = lock
            .get_index_of_bg(bg)
            .ok_or(anyhow!("Background is not in queue"))?;

        let mut target_index = lock
            .get_index_of_bg(target_bg)
            .ok_or(anyhow!("Target background is not in queue"))?;

        if bg_index == target_index {
            return Err(anyhow!("Refusing to move wallpaper to the same position"));
        }

        match before_or_after {
            Position::Before => {
                // Since we are removing bg, rightward items will shift leftward.
                // If target is rightward, we need to adjust the index
                if target_index > bg_index {
                    target_index -= 1;
                }
            }
            Position::After => {
                if target_index < bg_index {
                    target_index += 1;
                }
            }
        }

        if bg_index == target_index {
            return Err(anyhow!("Refusing to move wallpaper to the same position"));
        }

        let item = lock.remove(bg_index);
        lock.insert(target_index, item);

        let mut i_lock = self.current_index.lock().await;

        // Update current_index
        *i_lock = lock.get_index_of_bg(queued_bg).expect(
            "We have held the lock to queue, therefore queued_bg should be somewhere in the queue",
        );

        drop(lock);

        // TODO: If current_index is affected, update it

        Ok((bg_index, target_index))
    }
}

impl Queue {
    pub fn new(v: Option<Vec<String>>) -> Self {
        Self {
            v: v.unwrap_or_default(),
        }
    }
}
