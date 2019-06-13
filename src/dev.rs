use lazy_static::lazy_static;
use alloc::collections;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::sync::Arc;
use spin::Mutex;
use spin::RwLock;
use crate::nine_p;
use alloc::borrow::ToOwned;

pub struct DevDrivers {
    drivers: collections::BTreeMap<String, Arc<RwLock<Box<dyn nine_p::NinePServer>>>>
}

impl DevDrivers {
    pub fn new() -> Self {
        Self {
            drivers: collections::BTreeMap::new()
        }
    }

    pub fn insert(&mut self, name: &str, driver: Box<dyn nine_p::NinePServer>) {
        self.drivers.insert(name.to_owned(), Arc::new(RwLock::new(driver)));
    }

    pub fn get(&self, name: &str) -> Option<Arc<RwLock<Box<dyn nine_p::NinePServer>>>> {
        match self.drivers.get(name) {
            None => None,
            Some(d) => Some(d.clone())
        }
    }
}

lazy_static! {
    pub static ref DEV_DRIVERS: Mutex<DevDrivers> = Mutex::new(DevDrivers::new());
}

pub fn insert_dev_driver(name: &str, driver: Box<dyn nine_p::NinePServer>) {
    DEV_DRIVERS.lock().insert(name, driver);
}

pub fn get_dev_driver(name: &str) -> Option<Arc<RwLock<Box<dyn nine_p::NinePServer>>>> {
    DEV_DRIVERS.lock().get(name)
}