use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use super::{Queue, persistence::Sqlite};

pub struct PlaylistManager {
    active_queue: Arc<Mutex<Queue>>,
    // playlists: HashMap<String, Vec<String>>,
    pub daytime_dependent: Arc<Mutex<(Vec<String>, Vec<String>)>>,
}

impl PlaylistManager {
    pub fn new(queue: &Arc<Mutex<Queue>>, db: &Sqlite) -> Self {
        let (day, night) = db.read_day_night_queue().await;

        Self {
            active_queue: queue.clone(),
            daytime_dependent: (day, night),
        }
    }
}
