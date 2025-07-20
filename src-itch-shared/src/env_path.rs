use std::{
    env::{self, VarError},
    path::{Path, PathBuf},
};

pub struct EnvPath {
    p: PathBuf,
}

impl EnvPath {
    pub fn xdg_runtime_dir(join_path: &'static str) -> Result<Self, VarError> {
        let p = env::var("XDG_RUNTIME_DIR")?;
        let p = PathBuf::from(p).join(join_path);

        Ok(Self { p })
    }

    pub fn home(join_path: &'static str) -> Result<Self, VarError> {
        let p = env::var("HOME")?;
        let p = PathBuf::from(p).join(join_path);

        Ok(Self { p })
    }
}

impl AsRef<Path> for EnvPath {
    fn as_ref(&self) -> &Path {
        self.p.as_path()
    }
}
