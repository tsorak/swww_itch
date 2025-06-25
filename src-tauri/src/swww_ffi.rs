use std::process::Command;

pub fn set_background(path: &str) -> bool {
    Command::new("swww")
        .args([
            "img",
            path,
            "--transition-fps",
            "60",
            "--transition-type",
            "any",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
