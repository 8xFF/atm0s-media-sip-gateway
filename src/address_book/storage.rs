use std::{collections::HashMap, sync::Arc};

use spin::RwLock;

use crate::protocol::{AppInfo, PhoneNumber};

#[derive(Clone)]
pub struct AddressBookStorage {
    internal: Arc<RwLock<AddressBookStorageInternal>>,
}

impl AddressBookStorage {
    pub fn new(root_secret: &str) -> Self {
        Self {
            internal: Arc::new(RwLock::new(AddressBookStorageInternal {
                root_app: AppInfo {
                    app_id: "".to_owned(),
                    app_secret: root_secret.to_owned(),
                },
                app_ids: Default::default(),
                app_secrets: Default::default(),
                numbers: Default::default(),
            })),
        }
    }

    pub fn validate_app(&self, app_secret: &str) -> Option<AppInfo> {
        self.internal.read().validate_app(app_secret)
    }

    pub fn validate_phone(&self, remote: std::net::SocketAddr, from: &str, to: &str) -> Option<(AppInfo, PhoneNumber)> {
        self.internal.read().validate_phone(remote, from, to)
    }

    pub fn sync_apps(&self, new_apps: Vec<AppInfo>) {
        self.internal.write().sync_apps(new_apps);
    }

    pub fn sync_numbers(&self, new_numbers: Vec<PhoneNumber>) {
        self.internal.write().sync_numbers(new_numbers);
    }
}

struct AddressBookStorageInternal {
    root_app: AppInfo,
    app_ids: HashMap<String, AppInfo>,
    app_secrets: HashMap<String, AppInfo>,
    numbers: HashMap<String, PhoneNumber>,
}

impl AddressBookStorageInternal {
    pub fn validate_app(&self, app_secret: &str) -> Option<AppInfo> {
        if app_secret.eq(&self.root_app.app_secret) {
            return Some(self.root_app.clone());
        }
        self.app_secrets.get(app_secret).cloned()
    }

    pub fn validate_phone(&self, remote: std::net::SocketAddr, _from: &str, to: &str) -> Option<(AppInfo, PhoneNumber)> {
        let number = self.numbers.get(to)?;
        let app = if number.app_id == self.root_app.app_id {
            &self.root_app
        } else {
            self.app_ids.get(&number.app_id)?
        };
        for subnet in &number.subnets {
            if subnet.contains(&remote.ip()) {
                return Some((app.clone(), number.clone()));
            }
        }
        None
    }

    pub fn sync_apps(&mut self, new_apps: Vec<AppInfo>) {
        let pre_len = self.app_ids.len();
        self.app_ids.clear();
        self.app_secrets.clear();
        for app in new_apps {
            self.app_secrets.insert(app.app_secret.clone(), app.clone());
            self.app_ids.insert(app.app_id.clone(), app);
        }
        if self.app_ids.len() != pre_len {
            log::info!("[AddressBookStorage] apps len changed from {} to {}", pre_len, self.app_ids.len());
        }
    }

    pub fn sync_numbers(&mut self, new_numbers: Vec<PhoneNumber>) {
        let pre_len = self.numbers.len();
        self.numbers.clear();
        for number in new_numbers {
            self.numbers.insert(number.number.clone(), number);
        }
        if self.numbers.len() != pre_len {
            log::info!("[AddressBookStorage] numbers len changed from {} to {}", pre_len, self.numbers.len());
        }
    }
}
