use super::*;

pub mod new {
    use super::*;

    pub async fn check_enabled(db: &Sqlite) -> anyhow::Result<bool> {
        match query_as!(
            row_types::DayNightEnabled,
            "SELECT (enabled) FROM app_settings WHERE setting = \"day_night_playlist\""
        )
        .fetch_one(db.pool())
        .await
        {
            Ok(v) => {
                if let Some(n) = v.enabled {
                    return Ok(n != 0);
                };
                anyhow::bail!(
                    "Setting 'day_night_playlist' should have 'enabled' field set. Ensure migrations have been run (sqlx migrate run). Otherwise: Time to panic!"
                )
            }
            Err(err) => Err(err)?,
        }
    }

    pub async fn init_day_night_queue(
        db: &Sqlite,
    ) -> Result<
        (Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>),
        (Option<sqlx::Error>, Option<sqlx::Error>),
    > {
        let day = dba::get_day_queue(&db);
        let night = dba::get_night_queue(&db);

        let (day, night) = tokio::join!(day, night);

        if day.is_err() || night.is_err() {
            Err((day.err(), night.err()))
        } else {
            let day = SwapQueue {
                name: Some("DAY".to_string()),
                v: day.unwrap(),
            };

            let night = SwapQueue {
                name: Some("NIGHT".to_string()),
                v: night.unwrap(),
            };

            Ok((Arc::new(Mutex::new(day)), Arc::new(Mutex::new(night))))
        }
    }
}

pub mod change_schedule {
    pub enum Schedule {
        Day(u8, u8),
        Night(u8, u8),
    }
}
