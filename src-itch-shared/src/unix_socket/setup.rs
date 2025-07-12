use std::{env, path::PathBuf};

pub enum UnixSocketPath {
    RuntimeDir(&'static str),
    AbsolutePath(&'static str),
}

pub trait IntoUnixSocketPath {
    fn into_listen_path(self) -> UnixSocketPath;
}

impl IntoUnixSocketPath for &'static str {
    fn into_listen_path(self) -> UnixSocketPath {
        UnixSocketPath::AbsolutePath(self)
    }
}

impl IntoUnixSocketPath for UnixSocketPath {
    fn into_listen_path(self) -> UnixSocketPath {
        self
    }
}

impl UnixSocketPath {
    pub fn to_pathbuf(self) -> Result<PathBuf, env::VarError> {
        match self {
            UnixSocketPath::RuntimeDir(name) => {
                Ok(PathBuf::from(env::var("XDG_RUNTIME_DIR")?).join(name))
            }
            UnixSocketPath::AbsolutePath(path) => Ok(PathBuf::from(path)),
        }
    }
}
