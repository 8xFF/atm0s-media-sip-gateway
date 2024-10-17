use std::time::Duration;

use tokio::time::sleep;

use crate::protocol::{AppsSyncResponse, PhoneNumbersSyncResponse};

use super::AddressBookStorage;

pub struct AddressBookSync {
    numbers_url: String,
    apps_url: String,
    storage: AddressBookStorage,
    interval: Duration,
}

impl AddressBookSync {
    pub fn new(numbers_url: &str, apps_url: &str, interval: Duration, storage: AddressBookStorage) -> Self {
        Self {
            numbers_url: numbers_url.to_string(),
            apps_url: apps_url.to_string(),
            interval,
            storage,
        }
    }

    async fn sync(&mut self) -> reqwest::Result<()> {
        let res: PhoneNumbersSyncResponse = reqwest::ClientBuilder::default()
            .timeout(self.interval / 2)
            .build()
            .expect("Should build client")
            .get(&self.numbers_url)
            .send()
            .await?
            .json()
            .await?;
        self.storage.sync_numbers(res.numbers);
        let res: AppsSyncResponse = reqwest::ClientBuilder::default()
            .timeout(self.interval / 2)
            .build()
            .expect("Should build client")
            .get(&self.apps_url)
            .send()
            .await?
            .json()
            .await?;
        self.storage.sync_apps(res.apps);
        Ok(())
    }

    pub async fn run_loop(&mut self) {
        loop {
            if let Err(e) = self.sync().await {
                log::error!("[AddressBookSync] sync error {e:?}");
            }
            sleep(self.interval).await;
        }
    }
}
