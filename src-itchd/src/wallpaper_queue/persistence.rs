use tokio::fs;

mod sqlite;
pub use sqlite::Sqlite;

pub async fn open_or_make_db() -> anyhow::Result<Sqlite> {
    let db_path = crate::util::EnvPath::home(".local/state/itch/state.db")?;

    fs::create_dir_all(db_path.as_ref().parent().unwrap()).await?;

    Ok(Sqlite::new(db_path).await?)
}

#[allow(unused)]
pub trait PersistentStorage {
    async fn read_queue(&self) -> Vec<String>;
    async fn write_queue(&self, queue: &Vec<String>) -> Result<(), impl std::error::Error>;
    async fn close(&self);
}

impl PersistentStorage for Sqlite {
    async fn read_queue(&self) -> Vec<String> {
        self.read_queue().await
    }
    async fn write_queue(&self, queue: &Vec<String>) -> Result<(), impl std::error::Error> {
        self.write_queue(queue).await
    }
    async fn close(&self) {
        self.close().await
    }
}
