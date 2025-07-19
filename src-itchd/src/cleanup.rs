use std::path::PathBuf;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
    time::{Duration, Instant},
};

use crate::wallpaper_queue::WallpaperQueue;

pub struct Cleanup {
    pub unix_socket_path: PathBuf,
    pub wallpaper_queue: WallpaperQueue,
}

impl Cleanup {
    pub async fn bind_os_signals(self) {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        select! {
            _ = sigterm.recv() => println!("Received SIGTERM"),
            _ = sigint.recv() => println!("Received SIGINT"),
        }

        let wq = self.wallpaper_queue;

        let socket = async || match std::fs::remove_file(self.unix_socket_path) {
            Ok(_) => {
                println!("Removed socket")
            }
            Err(_) => {
                println!("Failed to remove socket")
            }
        };

        let scheduler = async || {
            let wq = wq.clone();

            println!("Shutting down Scheduler...");

            loop {
                if let Err(_no_receiver) = wq.scheduler.shutdown().await {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            println!("Shut down Scheduler");
        };

        let save_queue = async || {
            let wq = wq.clone();

            println!("Saving user-sorted queue...");

            let queue = wq.queue.lock().await;
            match wq.db.write_queue(queue.as_vec()).await {
                Ok(_) => println!("Saved user-sorted queue"),
                Err(err) => eprintln!("Failed to write some rows to sqlite database: {:#?}", err),
            }

            wq.db.close().await;

            println!("Saved user-sorted queue...");
        };

        let ((_, t1), (_, t2), (_, t3)) =
            tokio::join!(time_op(socket), time_op(scheduler), time_op(save_queue));

        println!("Cleanup process timings:");
        [("socket", t1), ("scheduler", t2), ("save_queue", t3)]
            .iter()
            .map(|(name, n)| (name, n.as_micros()))
            .for_each(|(name, n)| println!("{name}: {n}µs"));

        println!();
    }
}

async fn time_op<R>(f: impl AsyncFnOnce() -> R) -> (R, Duration) {
    let start = Instant::now();
    let r = f().await;
    (r, start.elapsed())
}
