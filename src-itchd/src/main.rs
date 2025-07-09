use anyhow::anyhow;

mod cleanup;
mod ipc;
mod wallpaper_queue;

use wallpaper_queue::WallpaperQueue;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cleanup::bind_os_signals();

    let bg_dir = std::env::home_dir()
        .ok_or(anyhow!("Could not get home directory"))?
        .join("backgrounds");

    let wallpaper_queue = WallpaperQueue::builder()
        .with_initial_queue_from_directory(&bg_dir)
        .await
        .dbg()
        .build();

    let args: Vec<String> = std::env::args().collect();
    if let Some(bg) = args.get(1).map(|s| s.to_string()) {
        let wq = wallpaper_queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let _ = wq.switch_to_wallpaper(&bg).await;
        });
    }

    ipc::run(wallpaper_queue).await;

    Ok(())
}
