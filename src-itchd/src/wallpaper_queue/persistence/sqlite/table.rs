use super::*;

pub trait DbTable<'a> {
    fn acquire(db: &'a Sqlite) -> Self;
}

impl Sqlite {
    pub fn table<'a, T: DbTable<'a>>(&'a self) -> T {
        T::acquire(self)
    }
}

pub struct DayNight<'a>(&'a Sqlite);
pub struct Queue<'a>(&'a Sqlite);

impl<'a> DbTable<'a> for DayNight<'a> {
    fn acquire(db: &'a Sqlite) -> Self {
        Self(db)
    }
}

impl<'a> DbTable<'a> for Queue<'a> {
    fn acquire(db: &'a Sqlite) -> Self {
        Self(db)
    }
}

//

pub mod impls {
    use super::*;

    use futures::future::join_all;

    impl DayNight<'_> {
        pub async fn insert_or_replace(
            &self,
            v: &Vec<String>,
            daytime: bool,
        ) -> Vec<Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>> {
            let ops = v.into_iter().enumerate().map(|(i, path)| {
                query("INSERT OR REPLACE INTO day_night_playlist (path, play_order, daytime) VALUES (?, ?, ?)")
                    .bind(path)
                    .bind(i as i64)
                    .bind(daytime)
                    .execute(self.0.pool())
            });

            // Write in batches to avoid timing out others waiting for a connection
            // loop {
            //     let batch = ops.take(5).peekable();

            //     if batch.peek().is_none() {
            //         break;
            //     }
            // }

            join_all(ops).await
        }
    }

    mod queue {
        use serde::Deserialize;
        use sqlx::FromRow;

        #[derive(Deserialize, FromRow)]
        pub struct LastQueueNameSetting {
            pub string: Option<String>,
        }
    }

    impl Queue<'_> {
        pub async fn read_state(&self) -> Result<crate::wallpaper_queue::Queue, sqlx::Error> {
            use super::super::row_types::QueuePathOnly;

            let name_query = query_as!(
                queue::LastQueueNameSetting,
                "SELECT (string) FROM app_settings WHERE setting = 'last_queue_name' LIMIT 1",
            )
            .fetch_one(self.0.pool());

            let queue_query = query_as!(
                QueuePathOnly,
                "SELECT path FROM Queue ORDER BY play_order ASC"
            )
            .fetch_all(self.0.pool());

            let (name, queue) = tokio::join!(name_query, queue_query);

            Ok(crate::wallpaper_queue::Queue {
                name: name.map(|strct| strct.string)?,
                v: queue.map(|vec| vec.into_iter().map(|strct| strct.path).collect())?,
            })
        }

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
            Option<Result<(), Vec<(usize, &'a String, sqlx::Error)>>>,
        ) {
            let name_query =
                query("REPLACE INTO app_settings (setting, string) VALUES ('last_queue_name', ?)")
                    .bind(q.name.as_deref().unwrap_or("NULL"))
                    .execute(self.0.pool());
            let empty_previous_queue_query = query("DELETE FROM queue").execute(self.0.pool());

            let queue_query = async {
                let mut errors = vec![];

                for (play_order, path) in q.v.iter().enumerate() {
                    let q = query("INSERT INTO Queue (path, play_order) VALUES (?, ?)")
                        .bind(path)
                        .bind(play_order as i64)
                        .execute(self.0.pool())
                        .await;

                    if let Some(err) = q.err().map(|err| (play_order, path, err)) {
                        errors.push(err);
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
}
