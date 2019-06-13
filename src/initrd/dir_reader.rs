use crate::nine_p;
use crate::tar;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;

#[derive(Debug)]
pub struct Reader<'a> {
    qid_pool: nine_p::qidpool::Pool,
    dir: Vec<tar::TarEntry<'a>>,
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(qid_pool: nine_p::qidpool::Pool, dir: Vec<tar::TarEntry<'a>>) -> Self {
        Self {
            qid_pool,
            dir,
            pos: 0,
        }
    }
}

impl Iterator for Reader<'_> {
    type Item = nine_p::dir::Dir;

    fn next(&mut self) -> Option<Self::Item> {
        match self.dir.get(self.pos) {
            Some(d) => {
                self.pos += 1;
                let h = d.header();
                let name = "/".to_owned() + &h.name;
                let atime = match h.atime {
                    Some(t) => t as u32,
                    None => 0
                };
                let qid = self.qid_pool.put(&name, super::tar_to_qid(h.typeflag.clone())).to_owned();
                Some(nine_p::dir::Dir::new(0, 0, &qid, 0, atime,
                                           h.mtime as u32, d.data().len() as u64,
                                           &name, h.uname.as_str(), h.gname.as_str(),
                                           h.uname.as_str()))
            }
            None => None
        }
    }
}

impl nine_p::RWC for Reader<'_> {}

impl nine_p::Read for Reader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> nine_p::Result<usize> {
        let dir = match self.next() {
            Some(d) => d,
            None => return Err(nine_p::DevError::EOF)
        };
        let dir_b = dir.as_bytes();
        if dir_b.len() > buf.len() {
            return Err(nine_p::DevError::SmallRead);
        }
        buf[..dir_b.len()].copy_from_slice(dir_b.as_slice());
        Ok(dir_b.len())
    }
}

impl nine_p::Write for Reader<'_> {
    fn write(&mut self, _buf: &[u8]) -> nine_p::Result<usize> {
        Err(nine_p::DevError::PermissionDenied)
    }

    fn flush(&mut self) -> nine_p::Result<()> {
        Ok(())
    }
}

impl nine_p::Close for Reader<'_> {
    fn close(&mut self) -> nine_p::Result<()> {
        Ok(())
    }
}