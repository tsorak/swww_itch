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

impl<'a> DbTable<'a> for DayNight<'a> {
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
}
