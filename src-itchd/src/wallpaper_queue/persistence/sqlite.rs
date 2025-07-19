use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{Pool, query, query_as};

#[derive(Debug, Clone)]
pub struct Sqlite {
    pool: Pool<sqlx::Sqlite>,
}

impl Sqlite {
    pub async fn new<P: AsRef<std::path::Path>>(file_path: P) -> Result<Sqlite, sqlx::Error> {
        let opts = SqliteConnectOptions::new()
            .create_if_missing(true)
            .in_memory(false)
            .filename(file_path);

        Ok(Self {
            pool: SqlitePool::connect_with(opts).await?,
        })
    }

    pub async fn read_queue(&self) -> Vec<String> {
        use row_types::*;

        query_as!(
            QueuePathOnly,
            "SELECT path FROM Queue ORDER BY play_order ASC"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|strct| strct.path)
        .collect()
    }

    pub async fn write_queue<'a>(
        &self,
        queue: &'a Vec<String>,
    ) -> Result<(), error::WriteQueueError> {
        let mut errors = vec![];

        for (play_order, path) in queue.iter().enumerate() {
            let q = query("INSERT OR REPLACE INTO Queue (path, play_order) VALUES (?, ?)")
                .bind(path)
                .bind(play_order as i64)
                .execute(&self.pool)
                .await;

            if let Err(err) = q {
                errors.push((play_order, path, err));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }

    pub async fn close(&self) {
        self.pool.close().await
    }
}

pub mod row_types {
    use serde::Deserialize;
    use sqlx::prelude::FromRow;

    #[derive(Debug, FromRow, Deserialize)]
    pub struct Queue {
        pub path: String,
        pub play_order: u32,
    }

    #[derive(Debug, FromRow, Deserialize)]
    pub struct QueuePathOnly {
        pub path: String,
    }
}

pub mod error {
    #[derive(Debug)]
    pub struct WriteQueueError(Vec<(usize, String, sqlx::Error)>);

    impl std::fmt::Display for WriteQueueError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // TODO: maybe make more descriptive
            write!(f, "WriteQueueError")
        }
    }

    impl std::error::Error for WriteQueueError {}

    impl From<Vec<(usize, &String, sqlx::Error)>> for WriteQueueError {
        fn from(value: Vec<(usize, &String, sqlx::Error)>) -> Self {
            Self(
                value
                    .into_iter()
                    .map(|(play_order, path, sqlx_err)| (play_order, path.to_owned(), sqlx_err))
                    .collect(),
            )
        }
    }
}
