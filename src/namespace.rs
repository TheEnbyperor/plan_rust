use alloc::collections;
use alloc::string::String;
use alloc::borrow::ToOwned;

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
}