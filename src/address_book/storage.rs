use std::{collections::HashMap, sync::Arc};

use spin::RwLock;

use crate::protocol::PhoneNumber;

#[derive(Default, Clone)]
pub struct AddressBookStorage {
    numbers: Arc<RwLock<HashMap<String, PhoneNumber>>>,
}

impl AddressBookStorage {
    pub fn allow(&self, remote: std::net::SocketAddr, _from: &str, to: &str) -> Option<PhoneNumber> {
        let numbers = self.numbers.read();
        let number = numbers.get(to)?;
        for subnet in &number.subnets {
            if subnet.contains(&remote.ip()) {
                return Some(number.clone());
            }
        }
        None
    }

    pub fn sync(&self, new_numbers: Vec<PhoneNumber>) {
        let mut numbers = self.numbers.write();
        let pre_len = numbers.len();
        numbers.clear();
        for number in new_numbers {
            numbers.insert(number.number.clone(), number);
        }
        if numbers.len() != pre_len {
            log::info!("[AddressBookStorage] numbers len changed from {} to {}", pre_len, numbers.len());
        }
    }
}
