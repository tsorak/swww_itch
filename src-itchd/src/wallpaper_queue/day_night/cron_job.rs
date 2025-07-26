use tokio::{
    task::JoinHandle,
    time::{Duration, sleep},
};

use super::*;

pub struct CronJob {
    handle: JoinHandle<()>,
}

impl CronJob {
    pub fn new(
        active_queue: Arc<Mutex<Queue>>,
        (day, night): (cron::Schedule, cron::Schedule),
        queues: (Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>),
        db: Sqlite,
    ) -> Self {
        let handle = tokio::spawn(async move { run(active_queue, (day, night), queues, db).await });

        Self { handle }
    }

    pub fn cancel(self) {
        self.handle.abort();
    }
}

async fn run(
    active_queue: Arc<Mutex<Queue>>,
    (day, night): (cron::Schedule, cron::Schedule),
    queues: (Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>),
    db: Sqlite,
) {
    loop {
        let now = chrono::Local::now();

        let next_day = day.after(&now).next().unwrap().signed_duration_since(&now);
        let next_night = night
            .after(&now)
            .next()
            .unwrap()
            .signed_duration_since(&now);

        let next_day = Duration::from_secs(next_day.num_seconds() as u64);
        let next_night = Duration::from_secs(next_night.num_seconds() as u64);

        tokio::select! {
            _ = sleep(next_day) => switch_active_queue_to("DAY", &queues, &active_queue, &db).await,
            _ = sleep(next_night) => switch_active_queue_to("NIGHT", &queues, &active_queue, &db).await,
        }
    }
}

pub async fn switch_active_queue_to(
    spec: &'static str,
    dnqs: &(Arc<Mutex<SwapQueue>>, Arc<Mutex<SwapQueue>>),
    active_queue: &Arc<Mutex<Queue>>,
    db: &Sqlite,
) {
    let q = match spec {
        "DAY" => &dnqs.0,
        "NIGHT" => &dnqs.1,
        _ => todo!("make enum for this"),
    };

    let (mut active_queue, q) = tokio::join!(active_queue.lock(), q.lock());

    let prev = active_queue.swap_with(&*q).await;

    prev.save_state(dnqs, db).await;
}
