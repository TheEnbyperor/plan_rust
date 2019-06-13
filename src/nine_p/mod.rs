pub mod qidpool;
pub mod dir;

use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use core::fmt::Debug;
use alloc::sync::Arc;
use spin::RwLock;
use core::cmp::min;

pub type Fid = u32;
pub const NO_FID: Fid = 0;

#[derive(Debug)]
pub enum DevError {
    EOF,
    AuthNotNeeded,
    PermissionDenied,
    FidInUse,
    NoFid,
    NoSeek,
    SmallRead,
    NoSuchFile,
    NotADir,
    FileOpen,
    Str(String),
}

pub type Result<T> = core::result::Result<T, DevError>;

impl DevError {
    fn description(&self) -> String {
        match &*self {
            DevError::EOF => "End of file".to_string(),
            DevError::AuthNotNeeded => "Authentication not required".to_string(),
            DevError::PermissionDenied => "Permission denied".to_string(),
            DevError::FidInUse => "Fid already in use".to_string(),
            DevError::NoFid => "Fid does not exit".to_string(),
            DevError::NoSeek => "File does not support seeking".to_string(),
            DevError::SmallRead => "Read size too small for data".to_string(),
            DevError::NoSuchFile => "No such file or directory".to_string(),
            DevError::NotADir => "Not a directory".to_string(),
            DevError::FileOpen => "File is open".to_string(),
            DevError::Str(string) => string.to_owned(),
        }
    }
}

#[derive(Debug)]
struct _Session {
    user: String,
    access: String,
}

#[derive(Debug, Clone)]
pub struct Session(Arc<RwLock<_Session>>);

impl Session {
    pub fn new(user: &str, access: &str) -> Self {
        Self(Arc::new(RwLock::new(_Session {
            user: user.to_owned(),
            access: access.to_owned()
        })))
    }
}

#[derive(Debug)]
struct _File<'a> {
    name: String,
    auth: bool,
    rwc: Option<Arc<RwLock<Box<(dyn FileRWC + 'a)>>>>
}

#[derive(Debug, Clone)]
pub struct File<'a>(Arc<RwLock<_File<'a>>>);

impl<'a> File<'a> {
    pub fn new(name: &str, auth: bool, rwc: Option<Box<(dyn FileRWC + 'a)>>) -> Self {
        Self(Arc::new(RwLock::new(_File {
            name: name.to_owned(),
            auth,
            rwc: match rwc {
                None => None,
                Some(rwc) => Some(Arc::new(RwLock::new(rwc)))
            }
        })))
    }

    pub fn name(&self) -> String {
        self.0.read().name.clone()
    }

    pub fn auth(&self) -> bool {
        self.0.read().auth
    }

    pub fn rwc(&self) -> Option< Arc<RwLock<Box<(dyn FileRWC + 'a)>>>> {
        self.0.write().rwc.clone()
    }

    pub fn set_rwc(&self, rwc: Box<(dyn FileRWC + 'a)>) {
        self.0.write().rwc = Some(Arc::new(RwLock::new(rwc)));
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum FileAccessMode {
    Read = 0,
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

pub trait Read: Debug {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

pub trait Write: Debug {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;
}

pub trait Seek: Debug {
    fn seek(&mut self, pos: u64) -> Result<u64>;
}

pub trait Close: Debug  {
    fn close(&mut self) -> Result<()>;
}

pub trait RWC: Read + Write + Close + Send + Sync {}
pub trait RWSC: RWC + Seek {}

pub trait FileRWC: Debug + Sync + Send {
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize>;
    fn write_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize>;
}

impl FileRWC for dyn RWSC {
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        self.seek(pos)?;
        self.read(buf)
    }

    fn write_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        self.seek(pos)?;
        self.write(buf)
    }
}

#[derive(Debug)]
pub struct RWCWrapper<'a> {
    rwc: Box<(dyn RWC + 'a)>,
    offset: u64
}


impl<'a> RWCWrapper<'a> {
    pub fn new(rwc: Box<(dyn RWC + 'a)>) -> Self {
        Self {
            rwc,
            offset: 0
        }
    }
}

impl FileRWC for RWCWrapper<'_> {
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        if self.offset != pos {
            return Err(DevError::NoSeek);
        }

        let n = self.rwc.read(buf)? as u64;
        self.offset += n;
        Ok(n as usize)
    }

    fn write_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        if self.offset != pos {
            return Err(DevError::NoSeek);
        }

        let n = self.rwc.write(buf)? as u64;
        self.offset += n;
        Ok(n as usize)
    }
}

impl FileRWC for &mut [u8] {
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        let pos = pos as usize;
        if pos >= self.len() {
            return Err(DevError::EOF);
        }

        let read_len = min(buf.len(), self.len() - pos);

        buf[..read_len].copy_from_slice(&self[pos..read_len]);
        Ok(read_len)
    }

    fn write_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        let pos = pos as usize;
        if pos >= self.len() {
            return Err(DevError::EOF);
        }

        let write_len = min(buf.len(), self.len() - pos);

        self[pos..write_len].copy_from_slice(&buf[..write_len]);
        Ok(write_len)
    }
}

impl FileRWC for &[u8] {
    fn read_at(&mut self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        let pos = pos as usize;
        if pos >= self.len() {
            return Err(DevError::EOF);
        }

        let read_len = min(buf.len(), self.len() - pos);

        buf[..read_len].copy_from_slice(&self[pos..read_len]);
        Ok(read_len)
    }

    fn write_at(&mut self, _pos: u64, _buf: &mut [u8]) -> Result<usize> {
        Err(DevError::PermissionDenied)
    }
}

pub trait NinePServer: Sync + Send {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn init(&self) {}
    fn shutdown(&self) {}
    fn reset(&self) {}

    fn auth(&mut self, afid: Fid, uname: &str, aname: &str) -> Result<qidpool::Qid>;
    fn attach(&mut self, fid: Fid, afid: Fid, uname: &str, aname: &str) -> Result<qidpool::Qid>;

    fn clunk(&mut self, fid: Fid) -> Result<()>;

    fn open(&mut self, fid: Fid, mode: &FileMode) -> Result<(qidpool::Qid, u32)>;
    fn create(&self, _fid: Fid, _name: &str, _perm: u32, _mode: &FileMode) -> Result<(qidpool::Qid, u32)> {
        Err(DevError::PermissionDenied)
    }

    fn walk(&mut self, fid: Fid, new_fid: Fid, names: &[&str]) -> Result<Vec<qidpool::Qid>>;

    fn read(&mut self, fid: Fid, offset: u64, count: usize) -> Result<Vec<u8>>;
    fn write(&self) -> Result<()> {
        Err(DevError::PermissionDenied)
    }

    fn remove(&mut self, fid: Fid) -> Result<()>;

    fn stat(&self) -> Result<()>;
    fn wstat(&self) -> Result<()> {
        Err(DevError::PermissionDenied)
    }
}

pub fn default_read(file: &mut File, offset: u64, count: usize) -> Result<Vec<u8>> {
    match file.rwc() {
        None => Err(DevError::Str(format!("File {} not open for reading", file.name()))),
        Some(rwc) => {
            let mut rwc = rwc.write();
            let mut buf: Vec<u8> = vec![0; count];
            let count = rwc.read_at(offset, buf.as_mut_slice())?;
            let buf = buf.as_slice()[..count].to_vec();
            Ok(buf)
        }
    }
}