mod dir_reader;

use x86_64::VirtAddr;
use core::slice;
use alloc::collections;
use crate::tar;
use crate::println;
use crate::nine_p;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::{ToString, String};


fn tar_to_qid(tar_type: tar::headers::FileType) -> nine_p::qidpool::QidType {
    match tar_type {
        tar::headers::FileType::Regular => nine_p::qidpool::QidType::FILE,
        tar::headers::FileType::Directory => nine_p::qidpool::QidType::DIRECTORY,
        _ => panic!("initrd can't handle this file type")
    }
}

#[derive(Debug)]
pub struct InitRD {
    start: VirtAddr,
    end: VirtAddr,
}

impl<'a> InitRD {
    pub fn new(start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            start,
            end,
        }
    }

    fn size(&self) -> u64 {
        return self.end - self.start;
    }

    fn data(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.start.as_ptr(), self.size() as usize) }
    }

    fn headers(&self) -> Vec<tar::TarEntry<'a>> {
        tar::find_headers(self.data())
    }

    pub fn dump(&self) {
        println!("{:?}", self.headers());
    }

    fn list_dir(&self, path: &str) -> Vec<tar::TarEntry<'a>> {
        if path == "/" {
            self.headers().into_iter().filter(|entry| !entry.header().name.contains("/")).collect()
        } else {
            self.headers().into_iter().filter(|entry| {
                let h = entry.header();
                let name = &h.name;
                if !name.starts_with(path) {
                    return false;
                } else {
                    let after: Vec<_> = name.split(path).collect();
                    let after = after[1..].into_iter().fold(String::new(), |a, b| a + path + b);
                    return !after.contains("/");
                }
            }).collect()
        }
    }

    pub fn stat(&self, path: &str) -> Option<tar::TarEntry<'a>> {
        if path == "/" {
            unimplemented!();
        } else {
            let file: Vec<tar::TarEntry> = self.headers().into_iter().filter(|entry| {
                let name = "/".to_owned() + &entry.header().name;
                name.as_str() == path
            }).collect();
            match file.len() {
                1 => Some(file[0].clone()),
                _ => None
            }
        }
    }
}

#[derive(Debug)]
pub struct InitRDServer<'a> {
    init_rd: InitRD,
    name: &'static str,
    description: &'static str,
    session_fid: collections::BTreeMap<nine_p::Fid, nine_p::Session>,
    qid_pool: nine_p::qidpool::Pool,
    files: collections::BTreeMap<nine_p::Fid, nine_p::File<'a>>,
}

impl<'a> InitRDServer<'a> {
    pub fn new(name: &'static str, description: &'static str, init_rd: InitRD) -> Self {
        Self {
            init_rd,
            name,
            description,
            session_fid: collections::BTreeMap::new(),
            qid_pool: nine_p::qidpool::Pool::new(),
            files: collections::BTreeMap::new(),
        }
    }

    fn check_fid(&self, fid: nine_p::Fid) -> nine_p::Result<()> {
        if !self.files.contains_key(&fid) {
            return Err(nine_p::DevError::NoFid);
        }

        if !self.files.contains_key(&fid) {
            return Err(nine_p::DevError::NoFid);
        }
        Ok(())
    }

    fn check_fid_in_use(&self, fid: nine_p::Fid) -> nine_p::Result<()> {
        if self.files.contains_key(&fid) {
            return Err(nine_p::DevError::FidInUse);
        }

        if self.files.contains_key(&fid) {
            return Err(nine_p::DevError::FidInUse);
        }
        Ok(())
    }
}

impl<'a> nine_p::NinePServer for InitRDServer<'a> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn auth(&mut self, _afid: nine_p::Fid, _uname: &str, _aname: &str) -> nine_p::Result<nine_p::qidpool::Qid> {
        Err(nine_p::DevError::AuthNotNeeded)
    }

    fn attach(&mut self, fid: nine_p::Fid, afid: nine_p::Fid, uname: &str, aname: &str) -> nine_p::Result<nine_p::qidpool::Qid> {
        if afid != 0 {
            return Err(nine_p::DevError::AuthNotNeeded);
        }

        let session = nine_p::Session::new(uname, aname);

        self.check_fid_in_use(fid)?;

        self.session_fid.insert(fid, session);
        self.files.insert(fid, nine_p::File::new("/", false, None));

        let qid = self.qid_pool.put("/", nine_p::qidpool::QidType::DIRECTORY);
        Ok(qid)
    }

    fn clunk(&mut self, fid: nine_p::Fid) -> nine_p::Result<()> {
        self.files.remove(&fid);
        self.session_fid.remove(&fid);
        Ok(())
    }

    fn open(&mut self, fid: nine_p::Fid, mode: &nine_p::FileMode) -> nine_p::Result<(nine_p::qidpool::Qid, u32)> {
        self.check_fid(fid)?;

        if mode.remove_on_close() {
            return Err(nine_p::DevError::PermissionDenied);
        }

        let file = self.files.get_mut(&fid).unwrap();
        let qid = self.qid_pool.get(&file.name()).unwrap();

        if mode.truncate() || mode.remove_on_close() {
            return Err(nine_p::DevError::PermissionDenied);
        }

        if qid.qid_type().contains(nine_p::qidpool::QidType::DIRECTORY) {
            if mode.access() != nine_p::FileAccessMode::Read {
                return Err(nine_p::DevError::PermissionDenied);
            }

            let entries = self.init_rd.list_dir(&file.name());
            let rwc = dir_reader::Reader::new(self.qid_pool.clone(), entries);
            let rwsc = nine_p::RWCWrapper::new(Box::new(rwc));
            file.set_rwc(Box::new(rwsc));
        } else {
            if !(mode.access() == nine_p::FileAccessMode::Read || mode.access() == nine_p::FileAccessMode::Execute) {
                return Err(nine_p::DevError::PermissionDenied);
            }

            let entry = self.init_rd.stat(&file.name()).unwrap();
            file.set_rwc(Box::new(entry.data()));
        }

        Ok((qid, 0))
    }

    fn walk(&mut self, fid: nine_p::Fid, new_fid: nine_p::Fid, names: &[&str]) -> nine_p::Result<Vec<nine_p::qidpool::Qid>> {
        self.check_fid(fid)?;

        if fid != new_fid {
            self.check_fid_in_use(new_fid)?;
        }

        let session = { self.session_fid.get(&fid).unwrap().clone() };
        let mut out_qid = Vec::<nine_p::qidpool::Qid>::new();
        let file = { self.files.get(&fid).unwrap().clone() };
        let qid = self.qid_pool.get(&file.name()).unwrap();
        let mut path = file.name();

        let err_exit = |err: nine_p::DevError, out: Vec<nine_p::qidpool::Qid>| {
            if out.len() == 0 {
                return Err(err);
            } else {
                return Ok(out);
            }
        };

        fn handle_element(path: &str, name: &str) -> String {
            if name != ".." {
                let mut path = path.to_string();
                path.push_str("/");
                path.push_str(name);
                path.to_string()
            } else {
                let elems: Vec<_> = path.split('/').collect();
                elems[..elems.len() - 1].into_iter().fold(String::new(), |a, b| a + "/" + b)
            }
        }

        if names.len() > 0 {
            path = path.trim_end_matches('/').to_string();
            if !qid.qid_type().contains(nine_p::qidpool::QidType::DIRECTORY) {
                return Err(nine_p::DevError::NotADir);
            } else if let Some(_) = file.rwc() {
                return Err(nine_p::DevError::FileOpen);
            } else {
                for name in names {
                    match out_qid.last() {
                        Some(q) => {
                            if q.qid_type().contains(nine_p::qidpool::QidType::DIRECTORY) {
                                return Ok(out_qid);
                            }
                        }
                        None => {}
                    }
                    path = handle_element(&path, name);
                    match self.init_rd.stat(&path) {
                        Some(e) => {
                            let qid = self.qid_pool.put(&path, tar_to_qid(e.header().typeflag.clone())).to_owned();
                            out_qid.push(qid);
                        }
                        None => return err_exit(nine_p::DevError::NoSuchFile, out_qid)
                    }
                }
            }
        }

        self.session_fid.insert(new_fid, session);
        self.files.insert(new_fid, nine_p::File::new(&path, false, None));

        Ok(out_qid)
    }

    fn read(&mut self, fid: nine_p::Fid, offset: u64, count: usize) -> nine_p::Result<Vec<u8>> {
        self.check_fid(fid)?;

        let file = self.files.get_mut(&fid).unwrap();
        nine_p::default_read(file, offset, count)
    }

    fn remove(&mut self, fid: nine_p::Fid) -> nine_p::Result<()> {
        self.files.remove(&fid);
        self.session_fid.remove(&fid);
        Err(nine_p::DevError::PermissionDenied)
    }

    fn stat(&self) -> nine_p::Result<()> {
        unimplemented!()
    }
}