pub mod background {
    use std::path::PathBuf;

    use anyhow::anyhow;

    pub fn canonicalize(name: &str) -> anyhow::Result<PathBuf> {
        Ok(std::env::home_dir()
            .ok_or(anyhow!("Home directory not found"))?
            .join("backgrounds")
            .join(name))
    }
}
