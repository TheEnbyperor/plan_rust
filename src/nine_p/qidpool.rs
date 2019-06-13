use alloc::collections;
use alloc::borrow::ToOwned;
use byteorder::{NetworkEndian, ByteOrder};
use alloc::string::String;
use alloc::sync::Arc;
use spin::RwLock;

pub type QidRaw = [u8; 13];

#[derive(Debug, Copy, Clone)]
pub struct Qid {
    qid_type: QidType,
    version: u32,
    path: u64,
}

impl Qid {
    pub fn new(qid_type: QidType, version: u32, path: u64) -> Self {
        Self {
            qid_type,
            version,
            path,
        }
    }

    pub fn as_bytes(&self) -> QidRaw {
        let mut qid = [0; 13];
        qid[0] = self.qid_type.bits();
        NetworkEndian::write_u32(&mut qid[1..5], self.version);
        NetworkEndian::write_u64(&mut qid[5..13], self.path);
        qid
    }

    pub fn qid_type(&self) -> QidType {
        self.qid_type
    }
}

bitflags! {
    pub struct QidType: u8 {
        const DIRECTORY = 0x80;
        const APPEND_ONLY = 0x40;
        const EXCLUSIVE_USE = 0x20;
        const AUTHENTICATION = 0x08;
        const TEMPORARY_FILE = 0x04;
        const FILE = 0x00;
    }
}

#[derive(Debug)]
struct _Pool {
    m: collections::BTreeMap<String, Qid>,
    path: u64,
}

#[derive(Debug, Clone)]
pub struct Pool(Arc<RwLock<_Pool>>);

impl Pool {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(_Pool {
            m: collections::BTreeMap::new(),
            path: 0,
        })))
    }

    pub fn put(&self, name: &str, qtype: QidType) -> Qid {
        let mut i = self.0.write();
        let path = i.path;
        i.path += 1;

        let qid: Qid = Qid::new(qtype, 0, path);

        if i.m.contains_key(name) {
            return *i.m.get(name).unwrap();
        } else {
            i.m.insert(name.to_owned(), qid);
            *i.m.get(name).unwrap()
        }
    }

    pub fn del(&self, name: &str) {
        self.0.write().m.remove(name);
    }

    pub fn get(& self, name: &str) -> Option<Qid> {
        match self.0.read().m.get(name) {
            Some(q) => Some(*q),
            None => None
        }
    }
}