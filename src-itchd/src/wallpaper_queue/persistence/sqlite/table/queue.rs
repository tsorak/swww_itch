use super::*;
use std::sync::Arc;

use serde::Deserialize;
use sqlx::FromRow;

#[derive(Deserialize, FromRow)]
pub struct LastQueueNameSetting {
    pub string: Option<String>,
}

impl Queue<'_> {
    // pub async fn read_state(&self) -> Result<crate::wallpaper_queue::Queue, sqlx::Error> {
    //     use super::super::row_types::QueuePathOnly;

    //     let name_query = query_as!(
    //         queue::LastQueueNameSetting,
    //         "SELECT (string) FROM app_settings WHERE setting = 'last_queue_name' LIMIT 1",
    //     )
    //     .fetch_one(self.0.pool());

    //     let queue_query = query_as!(
    //         QueuePathOnly,
    //         "SELECT path FROM Queue ORDER BY play_order ASC"
    //     )
    //     .fetch_all(self.0.pool());

    //     let (name, queue) = tokio::join!(name_query, queue_query);

    //     Ok(crate::wallpaper_queue::Queue {
    //         name: name.map(|strct| strct.string)?,
    //         v: queue.map(|vec| vec.into_iter().map(|strct| strct.path).collect())?,
    //     })
    // }

    /// Returns
    /// (_, _, None) => Failed to clear Queue table
    /// (_, _, Some(Err)) => One or more insertions failed
    /// (_, _, Some(Ok)) => All went well
    pub async fn replace_with<'a>(
        &self,
        q: &'a crate::wallpaper_queue::Queue,
    ) -> (
        Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>,
        Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>,
        Option<Result<(), Vec<(usize, Arc<String>, sqlx::Error)>>>,
    ) {
        let lock = q.current_playlist.lock().await;
        let last_queue_name = match str::from_utf8(lock.as_bytes()).unwrap() {
            "" => None,
            q => Some(q.to_owned()),
        };
        drop(lock);

        let name_query =
            query("REPLACE INTO app_settings (setting, string) VALUES ('last_queue_name', ?)")
                .bind(last_queue_name)
                .execute(self.0.pool());

        let empty_previous_queue_query = query("DELETE FROM queue").execute(self.0.pool());

        let queue_query = async {
            let mut errors = vec![];

            let lock = q.internal.map.lock().await;
            if let Some(vec) = lock.get("") {
                for (play_order, path) in vec.iter().enumerate() {
                    let q = query("INSERT INTO Queue (path, play_order) VALUES (?, ?)")
                        .bind(path.as_str())
                        .bind(play_order as i64)
                        .execute(self.0.pool())
                        .await;

                    if let Some(err) = q.err().map(|err| (play_order, path.clone(), err)) {
                        errors.push(err);
                    }
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        };

        // queue_query is dependent on empty_previous_queue_query to complete successfully.
        let stage_1 = tokio::join!(name_query, empty_previous_queue_query);

        if stage_1.1.is_err() {
            return (stage_1.0, stage_1.1, None);
        }

        let stage_2 = (stage_1.0, stage_1.1, Some(queue_query.await));

        stage_2
    }
}
