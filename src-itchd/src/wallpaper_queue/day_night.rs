use std::{str::FromStr, sync::Arc};

use sqlx::{query, query_as};
use tokio::sync::Mutex;

use crate::wallpaper_queue::SwapQueue;

use super::{Queue, Sqlite};

mod cron_job;
mod helper;
use cron_job::CronJob;
use helper::change_schedule::Schedule;

/// The queues being applied at Day/Night-time
pub type Queues = (Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>);

#[derive(Clone)]
pub struct DayNightQueue {
    // all_backgrounds: &'a Arc<Mutex<Vec<String>>>,
    enabled: Arc<Mutex<bool>>,
    active_queue: Arc<Mutex<Queue>>,
    cron: Arc<Mutex<Option<cron_job::CronJob>>>,
    day_night_queue: (Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>),
    schedule: (Arc<Mutex<cron::Schedule>>, Arc<Mutex<cron::Schedule>>),
    db: Sqlite,
}

impl DayNightQueue {
    pub async fn new(active_queue: Arc<Mutex<Queue>>, db: Sqlite) -> Self {
        let schedule = {
            let day = cron::Schedule::from_str("0 0 6 * * * *").unwrap();
            let night = cron::Schedule::from_str("0 0 18 * * * *").unwrap();

            (Arc::new(Mutex::new(day)), Arc::new(Mutex::new(night)))
        };

        let enabled = helper::new::check_enabled(&db).await.unwrap();

        let v = Self {
            enabled: Arc::new(Mutex::new(helper::new::check_enabled(&db).await.unwrap())),
            active_queue,
            cron: Arc::new(Mutex::new(None)),
            day_night_queue: helper::new::init_day_night_queue(&db).await.unwrap(),
            schedule,
            db,
        };

        if enabled {
            v.start_cron().await;
        }

        v
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.lock().await
    }

    /// Returns Ok(true) if the setting was changed from its previous state.
    pub async fn set_enabled(&self, b: bool) -> Result<bool, sqlx::Error> {
        let changed =
            query("UPDATE app_settings SET enabled = ? WHERE setting = \"day_night_playlist\"")
                .bind(b as i64)
                .execute(self.db.pool())
                .await
                .map(|v| v.rows_affected() > 0)?;

        if changed {
            if b {
                self.start_cron().await;
            } else {
                self.stop_cron().await;
            }

            let mut lock = self.enabled.lock().await;
            *lock = b;
            drop(lock);
        }

        Ok(changed)
    }

    pub async fn change_schedule(&self, spec: Schedule) {
        todo!();
        // let (h, m) = match &spec {
        //     Schedule::Day(h, m) => (h, m),
        //     Schedule::Night(h, m) => (h, m),
        // };

        // let cron_expr = format!("0 {m} {h} * * * *");
        // let schedule = cron::Schedule::from_str(&cron_expr).unwrap();

        // self.schedule
    }

    async fn start_cron(&self) {
        let mut job_lock = self.cron.lock().await;

        if job_lock.is_some() {
            return;
        }

        let schedule = {
            let (day, night) = tokio::join!(self.schedule.0.lock(), self.schedule.1.lock());

            (day.clone(), night.clone())
        };

        let job = CronJob::new(
            self.active_queue.clone(),
            schedule,
            self.day_night_queue.clone(),
            self.db.clone(),
        );

        *job_lock = Some(job);
    }

    async fn stop_cron(&self) {
        let mut job_lock = self.cron.lock().await;

        if let Some(job) = job_lock.take() {
            job.cancel();
        }
    }
}

mod row_types {
    use serde::Deserialize;
    use sqlx::FromRow;

    #[derive(Deserialize, FromRow)]
    pub struct DayNightEnabled {
        pub enabled: Option<i64>,
    }
}

mod dba {
    use super::Sqlite;
    use crate::wallpaper_queue::persistence::sqlite::row_types::QueuePathOnly;

    pub async fn get_day_queue(db: &Sqlite) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_as!(
            QueuePathOnly,
            "SELECT (path) FROM day_night_playlist WHERE daytime = 1 ORDER BY play_order ASC"
        )
        .fetch_all(db.pool())
        .await
        .map(|ok| ok.into_iter().map(|v| v.path).collect())
    }

    pub async fn get_night_queue(db: &Sqlite) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_as!(
            QueuePathOnly,
            "SELECT (path) FROM day_night_playlist WHERE daytime = 0 ORDER BY play_order ASC"
        )
        .fetch_all(db.pool())
        .await
        .map(|ok| ok.into_iter().map(|v| v.path).collect())
    }
}
