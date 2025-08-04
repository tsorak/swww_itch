use std::path::Path;

use swww_itch_shared::env_path::EnvPath;
use tokio::fs::DirEntry;

pub struct FsBackgrounds {
    vec: Vec<String>,
}

impl FsBackgrounds {
    pub async fn new() -> anyhow::Result<Self> {
        let mut dir = tokio::fs::read_dir(
            EnvPath::home("backgrounds")
                .as_ref()
                .map_err(|e| anyhow::anyhow!("Failed to get HOME directory. VarError: {e}"))?,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Could not read '$HOME/backgrounds'. Error: {e}"))?;

        let mut bg_paths = vec![];

        loop {
            match dir.next_entry().await {
                Ok(Some(entry)) => {
                    let _ = check_entry(entry, &mut bg_paths)
                        .await
                        .inspect_err(|err| eprintln!("{err}"));
                }
                Ok(None) => break,
                _ => continue,
            };
        }

        Ok(Self { vec: bg_paths })
    }

    pub fn into_inner(self) -> Vec<String> {
        self.vec
    }
}

async fn check_entry(e: DirEntry, bg_paths: &mut Vec<String>) -> anyhow::Result<()> {
    let f = e.file_type().await?;

    let mut entry_path = String::new();
    let load_entry_path = |v: &mut String| -> anyhow::Result<()> {
        let p = e.path();
        *v = p
            .as_path()
            .to_str()
            .ok_or(anyhow::anyhow!(
                "invalid unicode file path: {}",
                p.display()
            ))?
            .to_string();
        Ok(())
    };

    if f.is_dir() {
        check_subdirectory(
            load_entry_path(&mut entry_path).map(|_| &mut entry_path)?,
            bg_paths,
        )
        .await?;
    } else if (f.is_file() || f.is_symlink())
        && has_image_extension(load_entry_path(&mut entry_path).map(|_| &mut entry_path)?)?
    {
        bg_paths.push(entry_path);
    }

    Ok(())
}

async fn check_subdirectory(p: impl AsRef<Path>, bg_paths: &mut Vec<String>) -> anyhow::Result<()> {
    let mut dir = tokio::fs::read_dir(p)
        .await
        .map_err(|err| anyhow::anyhow!("Failed to traverse subdir: {err}"))?;

    loop {
        match dir.next_entry().await {
            Ok(Some(entry)) => {
                let _ = check_entry(entry, bg_paths);
            }
            Ok(None) => break,
            _ => continue,
        };
    }

    Ok(())
}

fn has_image_extension(p: impl AsRef<Path>) -> anyhow::Result<bool> {
    match p
        .as_ref()
        .extension()
        .ok_or(anyhow::anyhow!(
            "File has no extension. Can't verify if it is an image or not: {}",
            p.as_ref().display()
        ))?
        .to_str()
        .expect("Verify earlier")
    {
        "jpg" | "JPG" | "png" | "PNG" => Ok(true),
        _ => {
            eprintln!(
                "Did not recognize file as an image: {}",
                p.as_ref().display()
            );
            Ok(false)
        }
    }
}
