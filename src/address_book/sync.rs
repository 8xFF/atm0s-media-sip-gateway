use std::time::Duration;

use serde::Deserialize;
use tokio::time::sleep;

use crate::protocol::PhoneNumber;

use super::AddressBookStorage;

#[derive(Debug, Deserialize)]
struct SyncResponse {
    numbers: Vec<PhoneNumber>,
}

pub struct AddressBookSync {
    url: String,
    storage: AddressBookStorage,
    interval: Duration,
}

impl AddressBookSync {
    pub fn new(url: &str, interval: Duration, storage: AddressBookStorage) -> Self {
        Self {
            url: url.to_string(),
            interval,
            storage,
        }
    }

    async fn sync(&mut self) -> reqwest::Result<()> {
        let res: SyncResponse = reqwest::ClientBuilder::default()
            .timeout(self.interval / 2)
            .build()
            .expect("Should build client")
            .get(&self.url)
            .send()
            .await?
            .json()
            .await?;
        self.storage.sync(res.numbers);
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
