use x86_64::VirtAddr;
use core::slice;
use alloc::collections;
use crate::tar;
use crate::println;
use crate::nine_p;

#[derive(Debug)]
pub struct InitRD {
    start: VirtAddr,
    end: VirtAddr,
}

impl InitRD {
    pub fn new(start: VirtAddr, end: VirtAddr) -> Self {
        Self {
            start,
            end
        }
    }

    fn size(&self) -> u64 {
        return self.end - self.start
    }

    pub fn dump(&self) {
        let data = unsafe { slice::from_raw_parts(self.start.as_ptr(), self.size() as usize) };
        let headers = tar::find_headers(data);
        println!("{:?}", headers);
    }
}

#[derive(Debug)]
pub struct InitRDServer {
    init_rd: InitRD,
    name: &'static str,
    description: &'static str,
    session_fid: collections::BTreeMap<nine_p::Fid, nine_p::Session>,
    qid_pool: nine_p::qidpool::Pool,
    files: collections::BTreeMap<nine_p::Fid, nine_p::File>
}

impl InitRDServer {
    pub fn new(name: &'static str, description: &'static str, init_rd: InitRD) -> Self {
        Self {
            init_rd,
            name,
            description,
            session_fid: collections::BTreeMap::new(),
            qid_pool: nine_p::qidpool::Pool::new(),
            files: collections::BTreeMap::new()
        }
    }
}

impl nine_p::NinePServer for InitRDServer {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn auth(&mut self, _afid: nine_p::Fid, _uname: &str, _aname: &str) -> nine_p::Result<&nine_p::qidpool::Qid> {
        Err(nine_p::DevError::AuthNotNeeded)
    }

    fn attach(&mut self, fid: nine_p::Fid, afid: nine_p::Fid, uname: &str, aname: &str) -> nine_p::Result<&nine_p::qidpool::Qid> {
        if afid != 0 {
            return Err(nine_p::DevError::AuthNotNeeded);
        }

        let session = nine_p::Session::new(uname, aname);

        match self.session_fid.contains_key(&fid) {
            false => self.session_fid.insert(fid, session),
            true => return Err(nine_p::DevError::FidInUse)
        };

        match self.files.contains_key(&fid) {
            false => self.files.insert(fid, nine_p::File::new("/", false, None)),
            true => return Err(nine_p::DevError::FidInUse)
        };

        let qid = self.qid_pool.put("/", nine_p::qidpool::QidType::DIRECTORY);
        Ok(qid)
    }

    fn clunk(&mut self, fid: nine_p::Fid) -> nine_p::Result<()> {
        self.files.remove(&fid);
        self.session_fid.remove(&fid);
        Ok(())
    }

    fn open(&self, fid: nine_p::Fid, mode: &nine_p::FileMode) -> nine_p::Result<(&nine_p::qidpool::Qid, u32)> {
        if !self.files.contains_key(&fid) {
            return Err(nine_p::DevError::NoFid);
        }

        if !self.files.contains_key(&fid) {
            return Err(nine_p::DevError::NoFid);
        }

        if mode.remove_on_close() {
            return Err(nine_p::DevError::PermissionDenied)
        }

        let file = self.files.get(&fid).unwrap();
        let qid = self.qid_pool.get(file.name()).unwrap();

        if qid.qid_type().contains(nine_p::qidpool::QidType::DIRECTORY) {
            if mode.truncate() || mode.remove_on_close() {
                return Err(nine_p::DevError::PermissionDenied);
            }
            if mode.access() != nine_p::FileAccessMode::Read {
                return Err(nine_p::DevError::PermissionDenied);
            }

            unimplemented!();
        } else {
            unimplemented!();
        }
    }

    fn read(&self, _nbytes: usize) -> nine_p::Result<&[u8]> {
        Ok(&[0][..])
    }

    fn remove(&mut self, fid: nine_p::Fid) -> nine_p::Result<()> {
        self.files.remove(&fid);
        self.session_fid.remove(&fid);
        Err(nine_p::DevError::PermissionDenied)
    }

    fn stat(&self) -> nine_p::Result<()> {
        Ok(())
    }
}