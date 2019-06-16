use lazy_static::lazy_static;
use alloc::collections;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use spin::RwLock;
use crate::nine_p;

#[derive(Clone)]
pub struct FileServer {
    server: Arc<Mutex<Box<dyn nine_p::NinePServer>>>,
    fid_pool: FidPool,
}

impl FileServer {
    pub fn server(&self) -> Arc<Mutex<Box<dyn nine_p::NinePServer>>> {
        self.server.clone()
    }

    pub fn fid_pool(&self) -> &FidPool {
        &self.fid_pool
    }
}

struct DevDrivers {
    pub drivers: collections::BTreeMap<char, FileServer>
}

impl DevDrivers {
    pub fn new() -> Self {
        Self {
            drivers: collections::BTreeMap::new()
        }
    }
}

lazy_static! {
   static ref DEV_DRIVERS: RwLock<DevDrivers> = RwLock::new(DevDrivers::new());
}

struct _FidPoll {
    cur_fid: nine_p::Fid,
    available_fids: Vec<nine_p::Fid>
}

#[derive(Clone)]
pub struct FidPool(Arc<RwLock<_FidPoll>>);

impl FidPool {
    pub fn new() -> Self {
        Self (Arc::new(RwLock::new(_FidPoll {
            cur_fid: 1,
            available_fids: Vec::new()
        })))
    }

    pub fn get_fid(&self) -> nine_p::Fid {
        let mut pool = self.0.write();
        match pool.available_fids.pop() {
            Some(f) => f,
            None => {
                let f = pool.cur_fid;
                if pool.cur_fid == nine_p::Fid::max_value() {
                    panic!("No more FIDs left");
                }
                pool.cur_fid += 1;
                f
            }
        }
    }

    pub fn clunk_fid(&self, fid: nine_p::Fid) {
        {
            let mut pool = self.0.write();
            if !pool.available_fids.contains(&fid) {
                pool.available_fids.push(fid);
            }
        }
        self.clear_vec();
    }

    fn clear_vec(&self) {
        let pos = {
            let pool = self.0.read();
            pool.available_fids.iter().position(|x| *x == (pool.cur_fid - 1))
        };
        match pos {
            Some (i) => {
                {
                    let mut pool = self.0.write();
                    pool.cur_fid -= 1;
                    pool.available_fids.remove(i);
                }
                self.clear_vec();
            }
            None => {}
        };
    }
}

pub fn insert_dev_driver(driver: Box<dyn nine_p::NinePServer>) {
    DEV_DRIVERS.write().drivers.insert(driver.name(), FileServer {
        server: Arc::new(Mutex::new(driver)),
        fid_pool: FidPool::new()
    });
}

pub fn get_dev_driver(name: char) -> Option<FileServer> {
    match DEV_DRIVERS.read().drivers.get(&name) {
        Some(d) => Some(d.clone()),
        None => None
    }
}