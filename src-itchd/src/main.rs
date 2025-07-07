use swww_itch_shared::{
    swww_ffi,
    unix_socket::{Request, Response, setup_listener},
};

mod cleanup;

#[tokio::main]
async fn main() {
    cleanup::bind_os_signals();

    let mut listener = setup_listener().await;

    println!("Waiting for connections...");
    loop {
        if let Some(mut c) = listener.recv().await {
            match c.take_request() {
                Request::SwitchToBackground(p) => {
                    println!(r#"Received job: SwitchToBackground("{p}")"#);
                    let success = swww_ffi::set_background(&p);
                    let _ = c
                        .respond(Response::SwitchToBackground(success))
                        .inspect_err(|err| eprintln!("Failed to send response: {err}"));
                }
                _ => {}
            }
        }
    }
}
