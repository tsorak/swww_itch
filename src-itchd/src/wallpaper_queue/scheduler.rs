use std::ops::ControlFlow;

use tokio::time::{Duration, Instant, sleep_until};

use super::*;
use swww_itch_shared::swww_ffi;

#[derive(Clone)]
pub enum Command {
    Interval(Duration),
    Index(usize),
    Shutdown,
}

impl Scheduler {
    pub fn start(queue: Arc<Mutex<Queue>>, current_index: Arc<Mutex<usize>>) -> SchedulerRemote {
        let (command_tx, command_rx) = mpsc::channel(8);
        let scheduler = Scheduler {
            queue,
            command_rx,
            interval: Duration::from_secs(60 * 60),
            current_index,
        };

        tokio::spawn(scheduler.run());

        SchedulerRemote { command_tx }
    }

    async fn run(mut self) {
        let start = Instant::now();
        let mut timeout = start + self.interval;
        let (tx, mut end_timeout_rx) = mpsc::channel::<()>(1);
        let end_timeout = async move || tx.send(()).await;

        let reset_timeout = |timeout: &mut Instant, to: &Duration| {
            *timeout = Instant::now() + *to;
        };

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => if self.handle_command(command, &end_timeout).await.is_break() {
                    break;
                },
                _ = Self::first(sleep_until(timeout), end_timeout_rx.recv()) => {
                    self.do_interval_task().await;
                    reset_timeout(&mut timeout, &self.interval);
                },
            }
        }
    }

    async fn first<R1, R2>(
        fut1: impl Future<Output = R1>,
        fut2: impl Future<Output = R2>,
    ) -> (Option<R1>, Option<R2>) {
        tokio::select! {
            r = fut1 => (Some(r), None),
            r = fut2 => (None, Some(r)),
        }
    }

    async fn handle_command<T, E>(
        &mut self,
        command: Command,
        end_timeout: &impl AsyncFn() -> Result<T, E>,
    ) -> ControlFlow<(), ()> {
        match command {
            Command::Interval(interval) => self.interval = interval,
            Command::Index(index) => {
                *self.current_index.lock().await = index;
                let _ = end_timeout().await;
            }
            Command::Shutdown => return ControlFlow::Break(()),
        }
        ControlFlow::Continue(())
    }

    async fn do_interval_task(&self) {
        let queue = self.queue.lock().await;

        let mut index = self.current_index.lock().await;

        let maybe_wallpaper = {
            if let Some(wallpaper) = queue.v.get(*index) {
                Some(wallpaper)
            } else if let Some(wallpaper) = queue.v.last() {
                Some(wallpaper)
            } else {
                None
            }
        };

        if let Some(wallpaper) = maybe_wallpaper {
            println!("[wq::scheduler.rs] {wallpaper}");

            swww_ffi::set_background(wallpaper).await;
        }

        if *index == queue.v.len() - 1 {
            *index = 0;
        } else {
            *index += 1;
        }
    }
}

impl SchedulerRemote {
    pub async fn reset_timeout_and_set_index(
        &self,
        index: usize,
    ) -> Result<(), mpsc::error::SendError<Command>> {
        self.command_tx.send(Command::Index(index)).await
    }
}
