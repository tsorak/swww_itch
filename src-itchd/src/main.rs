use anyhow::anyhow;

use swww_itch_shared::unix_socket::{UnixSocketPath, setup_listener};

mod cleanup;
mod ipc;
mod util;
mod wallpaper_queue;

use cleanup::Cleanup;
use wallpaper_queue::WallpaperQueue;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bg_dir = std::env::home_dir()
        .ok_or(anyhow!("Could not get home directory"))?
        .join("backgrounds");

    let wallpaper_queue = WallpaperQueue::builder()
        .with_ordered_queue()
        .await
        .dbg_queue()
        .with_initial_queue_from_directory(&bg_dir)
        .await
        .dbg_queue()
        .build()
        .await;

    let args: Vec<String> = std::env::args().collect();
    if let Some(bg) = args.get(1).map(|s| s.to_string()) {
        let wq = wallpaper_queue.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let _ = wq.switch_to_wallpaper(&bg).await;
        });
    }

    let unix_socket_path = UnixSocketPath::RuntimeDir("swwwitch.sock").to_pathbuf()?;
    let socket = setup_listener(&unix_socket_path).await;
    let wq = wallpaper_queue.clone();
    tokio::select! {
        _ = Cleanup { unix_socket_path, wallpaper_queue: wq }.bind_os_signals() => {},
        _ = ipc::run(socket, wallpaper_queue) => {},
    }

    Ok(())
}
