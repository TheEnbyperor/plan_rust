pub mod qidpool;

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};

pub type Fid = u32;
pub const NO_FID: Fid = 0;

#[derive(Debug)]
pub enum DevError {
    AuthNotNeeded,
    PermissionDenied,
    FidInUse,
    NoFid,
    Str(String),
}

pub type Result<T> = core::result::Result<T, DevError>;

impl DevError {
    fn description(&self) -> String {
        match &*self {
            DevError::AuthNotNeeded => "Authentication not required".to_string(),
            DevError::PermissionDenied => "Permission denied".to_string(),
            DevError::FidInUse => "Fid already in use".to_string(),
            DevError::NoFid => "Fid does not exit".to_string(),
            DevError::Str(string) => string.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct Session {
    user: String,
    access: String,
}

impl Session {
    pub fn new(user: &str, access: &str) -> Self {
        Self {
            user: user.to_owned(),
            access: access.to_owned()
        }
    }
}

#[derive(Debug)]
pub struct File {
    name: String,
    auth: bool,
}

impl File {
    pub fn new(name: &str, auth: bool) -> Self {
        Self {
            name: name.to_owned(),
            auth
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum FileAccessMode {
    Read= 0,
    Write = 1,
    ReadWrite = 2,
    Execute = 3
}

pub struct FileMode {
    access: FileAccessMode,
    truncate: bool,
    remove_on_close: bool
}

impl FileMode {
    pub fn access(&self) -> FileAccessMode {
        self.access
    }

    pub fn truncate(&self) -> bool {
        self.truncate
    }

    pub fn remove_on_close(&self) -> bool {
        self.remove_on_close
    }
}

impl FileMode {
    pub fn new(access: FileAccessMode, truncate: bool, remove_on_close: bool) -> Self {
        Self {
            access,
            truncate,
            remove_on_close
        }
    }
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}

pub trait NinePServer {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn init(&self) {}
    fn shutdown(&self) {}
    fn reset(&self) {}

    fn auth(&mut self, afid: Fid, uname: &str, aname: &str) -> Result<&qidpool::Qid>;
    fn attach(&mut self, fid: Fid, afid: Fid, uname: &str, aname: &str) -> Result<&qidpool::Qid>;

    fn clunk(&mut self, fid: Fid) -> Result<()>;

    fn open(&self, fid: Fid, mode: &FileMode) -> Result<(&qidpool::Qid, u32)>;
    fn create(&self, fid: Fid, name: &str, perm: u32, mode: &FileMode) -> Result<(&qidpool::Qid, u32)> {
        Err(DevError::PermissionDenied)
    }

    fn read(&self, nbytes: usize) -> Result<&[u8]>;
    fn write(&self) -> Result<()> {
        Err(DevError::PermissionDenied)
    }

    fn remove(&mut self, fid: Fid) -> Result<()>;

    fn stat(&self) -> Result<()>;
    fn wstat(&self) -> Result<()> {
        Err(DevError::PermissionDenied)
    }
}