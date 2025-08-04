// called wallpaper here instead of background to not be confused with the verb background.

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use anyhow::anyhow;
use swww_itch_shared::{message::Position, swww_ffi};
use tokio::{
    sync::{Mutex, mpsc},
    time::Duration,
};

mod builder;
mod day_night;
mod persistence;
// mod playlist;
mod fs_backgrounds;
mod internal;
mod scheduler;

pub use builder::WallpaperQueueBuilder;
use day_night::DayNightQueue;
pub use persistence::sqlite::table;
use persistence::{Sqlite, open_or_make_db};
use scheduler as sch;

#[derive(Clone)]
pub struct WallpaperQueue {
    pub queue: Queue,
    pub scheduler: SchedulerRemote,
    pub db: Sqlite,
    pub day_night_queue: DayNightQueue,
}

#[derive(Clone)]
pub struct Queue {
    current_playlist: Arc<Mutex<Cow<'static, str>>>,
    current_index: Arc<Mutex<usize>>,
    internal: internal::Internal,
}

struct Scheduler {
    queue: Queue,
    command_rx: mpsc::Receiver<sch::Command>,
    interval: Duration,
}

#[derive(Clone)]
pub struct SchedulerRemote {
    command_tx: mpsc::Sender<sch::Command>,
}

impl WallpaperQueue {
    pub fn builder() -> WallpaperQueueBuilder {
        WallpaperQueueBuilder::new()
    }

    pub async fn new(queue: Queue, db: Option<Sqlite>) -> Self {
        let db = db.unwrap_or(
            open_or_make_db()
                .await
                .inspect_err(|err| eprintln!("Error: {err}"))
                .unwrap(),
        );

        let dnq = DayNightQueue::new(queue.clone(), db.clone()).await;

        Self {
            queue: queue.clone(),
            scheduler: Scheduler::start(queue),
            db,
            day_night_queue: dnq,
        }
    }

    pub async fn get_queue(&self) -> Vec<(Cow<'_, str>, Vec<Arc<String>>)> {
        self.queue
            .internal
            .map
            .lock()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub async fn switch_to_wallpaper(&self, bg: &str) -> anyhow::Result<()> {
        let lock = self.queue.internal.map.lock().await;

        lock.get("ALL")
            .unwrap()
            .iter()
            .find(|p| p.as_str() == bg)
            .ok_or(anyhow::anyhow!("Specified background does not exist"))?;

        drop(lock);

        swww_ffi::set_background(bg);

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
    pub fn new(playlists: HashMap<Cow<'static, str>, Vec<Arc<String>>>) -> Self {
        Self {
            current_index: Arc::new(Mutex::new(0)),
            current_playlist: Arc::new(Mutex::new("".into())),
            internal: internal::Internal::new(playlists),
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
