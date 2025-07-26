// called wallpaper here instead of background to not be confused with the verb background.

use std::sync::Arc;

use anyhow::anyhow;
use swww_itch_shared::message::Position;
use tokio::{
    sync::{Mutex, mpsc},
    time::Duration,
};

mod builder;
mod day_night;
mod persistence;
// mod playlist;
mod scheduler;

pub use builder::WallpaperQueueBuilder;
use day_night::DayNightQueue;
use persistence::{Sqlite, open_or_make_db};
use scheduler as sch;

#[derive(Clone)]
pub struct WallpaperQueue {
    pub queue: Arc<Mutex<Queue>>,
    pub scheduler: SchedulerRemote,
    pub current_index: Arc<Mutex<usize>>,
    pub db: Sqlite,
    pub day_night_queue: DayNightQueue,
}

pub struct Queue {
    name: Option<String>,
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

    pub async fn new(initial_queue: Vec<String>, db: Option<Sqlite>) -> Self {
        let queue = Arc::new(Mutex::new(Queue::new(Some(initial_queue))));
        let current_index = Arc::new(Mutex::new(0));
        let db = db.unwrap_or(
            open_or_make_db()
                .await
                .inspect_err(|err| eprintln!("Error: {err}"))
                .unwrap(),
        );

        let dnq = DayNightQueue::new(queue.clone(), db.clone()).await;

        Self {
            queue: queue.clone(),
            scheduler: Scheduler::start(queue, current_index.clone()),
            current_index,
            db,
            day_night_queue: dnq,
        }
    }

    pub async fn get_queue(&self) -> Vec<String> {
        self.queue.lock().await.v.to_owned()
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

    pub async fn rearrange_wallpaper(
        &self,
        bg: &str,
        before_or_after: &Position,
        target_bg: &str,
    ) -> anyhow::Result<(usize, usize)> {
        let i_lock = self.current_index.lock().await;
        let mut lock = self.queue.lock().await;

        let queued_bg = lock
            .v
            .get(*i_lock)
            .expect("current_index should always point to an existing item")
            .to_owned();
        drop(i_lock);

        let bg_index = lock
            .v
            .iter()
            .position(|v| v.ends_with(bg))
            .ok_or(anyhow!("Background is not in queue"))?;

        let mut target_index = lock
            .v
            .iter()
            .position(|v| v.as_str().ends_with(target_bg))
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

        let item = lock.v.remove(bg_index);
        lock.v.insert(target_index, item);

        let mut i_lock = self.current_index.lock().await;

        // Update current_index
        *i_lock = lock
            .v
            .iter()
            .enumerate()
            .find_map(|(i, v)| {
                if v.as_str() == queued_bg.as_str() {
                    Some(i)
                } else {
                    None
                }
            })
            .expect("We have held the lock to queue, therefore queued_bg should be somewhere in the queue");

        drop(lock);

        Ok((bg_index, target_index))
    }
}

impl Queue {
    pub fn new(v: Option<Vec<String>>) -> Self {
        Self {
            name: None,
            v: v.unwrap_or_default(),
        }
    }

    pub fn as_vec(&self) -> &Vec<String> {
        &self.v
    }

    /// Swaps out the current queue with "with".
    /// Returns the previous one.
    pub(self) async fn swap_with(&mut self, with: &SwapQueue) -> SwapQueue {
        let prev = SwapQueue {
            name: self.name.take(),
            v: std::mem::take(&mut self.v),
        };

        self.name = with.name.clone();
        self.v = with.v.clone();

        prev
    }
}

struct SwapQueue {
    name: Option<String>,
    v: Vec<String>,
}

impl SwapQueue {
    pub async fn save_state(self, (day, night): &day_night::Queues, db: &Sqlite) {
        use persistence::sqlite::table::DayNight;

        if let Some(playlist) = self.name.as_deref() {
            match playlist {
                "DAY" | "NIGHT" => {
                    let daytime = playlist == "DAY";
                    db.table::<DayNight>()
                        .insert_or_replace(&self.v, daytime)
                        .await;

                    if daytime {
                        let _old = std::mem::replace(&mut day.lock().await.v, self.v);
                    } else {
                        let _old = std::mem::replace(&mut night.lock().await.v, self.v);
                    }
                }
                _playlist => {
                    todo!()
                }
            };
        }
    }
}
