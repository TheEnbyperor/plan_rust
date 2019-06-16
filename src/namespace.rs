use alloc::collections;
use alloc::string::String;
use alloc::borrow::ToOwned;
use crate::dev;
use crate::nine_p;

pub struct Namespace {
    binds: collections::BTreeMap<String, String>
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            binds: collections::BTreeMap::new()
        }
    }

    pub fn bind(&mut self, src: &str, dst: &str) {
        self.binds.insert(src.to_owned(), dst.to_owned());
    }

    pub fn open_file(&self, path: &str) -> nine_p::Result<(dev::FileServer, nine_p::Fid)> {
        if path.starts_with("#") {
            match path.chars().nth(1) {
                Some(c) => {
                    match dev::get_dev_driver(c) {
                        Some(d) => {
                            let fid = d.fid_pool().get_fid();
                            d.server().lock().attach(fid, nine_p::NO_FID, "", "")?;
                            Ok((d, fid))
                        },
                        None => Err(nine_p::DevError::NoSuchFile)
                    }
                },
                None => Err(nine_p::DevError::NoSuchFile)
            }
        } else {
            unimplemented!()
        }
    }
}