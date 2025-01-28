use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2_foundation::{NSDictionary, NSNumber, NSString};

pub trait NSDictionaryExtensions {
    fn get_i32_for_key(&self, key: &str) -> Option<i32>;
    fn get_string_for_key(&self, key: &str) -> Option<String>;

    fn get_f64_for_key(&self, key: &str) -> Option<f64>;

    #[allow(dead_code)]
    fn get_bool_for_key(&self, key: &str) -> Option<bool>;
}

impl NSDictionaryExtensions for NSDictionary<NSString, NSObject> {
    fn get_string_for_key(&self, key: &str) -> Option<String> {
        match &self.objectForKey(&*NSString::from_str(key)) {
            Some(value) => Option::from(unsafe {
                Retained::cast_unchecked::<NSString>(value.to_owned()).to_string()
            }),
            None => None,
        }
    }

    fn get_f64_for_key(&self, key: &str) -> Option<f64> {
        match &self.objectForKey(&*NSString::from_str(key)) {
            Some(value) => Option::from(unsafe {
                Retained::cast_unchecked::<NSNumber>(value.to_owned()).as_f64()
            }),
            None => None,
        }
    }

    fn get_i32_for_key(&self, key: &str) -> Option<i32> {
        match &self.objectForKey(&*NSString::from_str(key)) {
            Some(value) => Option::from(unsafe {
                Retained::cast_unchecked::<NSNumber>(value.to_owned()).as_i32()
            }),
            None => None,
        }
    }

    fn get_bool_for_key(&self, key: &str) -> Option<bool> {
        match &self.objectForKey(&*NSString::from_str(key)) {
            Some(value) => Option::from(unsafe {
                Retained::cast_unchecked::<NSNumber>(value.to_owned()).as_bool()
            }),
            None => None,
        }
    }
}
