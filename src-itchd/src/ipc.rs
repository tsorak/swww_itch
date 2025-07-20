use swww_itch_shared::{
    message::{Request, Response},
    unix_socket::UnixSocket,
};

use crate::wallpaper_queue::WallpaperQueue;

pub async fn run(mut listener: UnixSocket<Request, ()>, wq: WallpaperQueue) {
    println!("[ipc.rs]: Waiting for connections...");
    while let Some(mut c) = listener.recv().await {
        match c.take_request() {
            Request::SwitchToBackground(p) => {
                println!(r#"Received job: SwitchToBackground("{p}")"#);

                let success = wq.switch_to_wallpaper(&p).await.is_ok();
                let _ = c
                    .respond(Response::SwitchToBackground(success))
                    .await
                    .inspect_err(|err| eprintln!("Failed to send response: {err}"));
            }
            Request::RearrangeBackground((bg, before_or_after, target_bg)) => {
                println!(
                    r#"Received job: RearrangeBackground("{bg}", "{before_or_after}", "{target_bg}")"#
                );

                let response = wq
                    .rearrange_wallpaper(&bg, &before_or_after, &target_bg)
                    .await
                    .map(|(move_index, to_index)| (true, move_index, to_index));

                let _ = c
                    .respond(Response::RearrangeBackground(
                        response
                            .inspect_err(|err| eprintln!("Failed to rearrange: {err}"))
                            .unwrap_or((false, 0, 0)),
                    ))
                    .await
                    .inspect_err(|err| eprintln!("Failed to send response: {err}"));
            }
            Request::GetQueue => {
                println!("Received job: GetQueue");

                let queue = wq.get_queue().await;

                let _ = c
                    .respond(Response::GetQueue(queue))
                    .await
                    .inspect_err(|err| eprintln!("Failed to send response: {err}"));
            }
        }
    }
}
