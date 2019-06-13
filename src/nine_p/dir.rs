use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use byteorder::{LittleEndian, ByteOrder};
use alloc::borrow::ToOwned;

pub struct Dir {
    dir_type: u16,
    dev: u32,
    qid: super::qidpool::Qid,
    mode: u32,
    atime: u32,
    mtime: u32,
    length: u64,
    name: String,
    uid: String,
    gid: String,
    muid: String,
}

fn write_u16(vec: &mut Vec<u8>, n: u16) {
    let mut buf = [0; 2];
    LittleEndian::write_u16(&mut buf, n);
    vec.extend_from_slice(&buf);
}

fn write_u32(vec: &mut Vec<u8>, n: u32) {
    let mut buf = [0; 4];
    LittleEndian::write_u32(&mut buf, n);
    vec.extend_from_slice(&buf);
}

fn write_u64(vec: &mut Vec<u8>, n: u64) {
    let mut buf = [0; 8];
    LittleEndian::write_u64(&mut buf, n);
    vec.extend_from_slice(&buf);
}

impl Dir {
    pub fn new(dir_type: u16, dev: u32, qid: &super::qidpool::Qid, mode: u32, atime: u32, mtime: u32, length: u64, name: &str, uid: &str, gid: &str, muid: &str) -> Self {
        Self {
            dir_type,
            dev,
            qid: *qid,
            mode,
            atime,
            mtime,
            length,
            name: name.to_owned(),
            uid: uid.to_owned(),
            gid: gid.to_owned(),
            muid: muid.to_owned()
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let name_len = self.name.len() as u16;
        let uid_len = self.uid.len() as u16;
        let gid_len = self.gid.len() as u16;
        let muid_len = self.muid.len() as u16;
        let len = 48 + name_len + uid_len + gid_len + muid_len;

        let mut out = vec![];

        write_u16(&mut out, len);
        write_u16(&mut out, self.dir_type);
        write_u32(&mut out, self.dev);
        out.extend_from_slice(&self.qid.as_bytes());
        write_u32(&mut out, self.mode);
        write_u32(&mut out, self.atime);
        write_u32(&mut out, self.mtime);
        write_u64(&mut out, self.length);

        write_u16(&mut out, name_len);
        out.extend_from_slice(&self.name.clone().into_bytes());

        write_u16(&mut out, uid_len);
        out.extend_from_slice(&self.uid.clone().into_bytes());

        write_u16(&mut out, gid_len);
        out.extend_from_slice(&self.gid.clone().into_bytes());

        write_u16(&mut out, muid_len);
        out.extend_from_slice(&self.muid.clone().into_bytes());

        out
    }
}