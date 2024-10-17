use jwt_simple::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    protocol::{AppId, CallDirection, InternalCallId, NotifyIdentify},
    AddressBookStorage,
};

const CALL_ISSUER: &str = "call";
const NOTIFY_ISSUER: &str = "noti";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct CallToken {
    pub direction: CallDirection,
    pub call_id: InternalCallId,
}

pub struct SecureContext {
    address_book: AddressBookStorage,
    key: HS256Key,
}

impl SecureContext {
    pub fn new(secret: &str, address_book: AddressBookStorage) -> Self {
        Self {
            address_book,
            key: HS256Key::from_bytes(secret.as_bytes()),
        }
    }

    pub fn check_secret(&self, secret: &str) -> Option<AppId> {
        let app = self.address_book.validate_app(secret)?;
        Some(app.app_id.into())
    }

    pub fn encode_call_token(&self, token: CallToken, duration_secs: u64) -> String {
        self.encode_token(token, CALL_ISSUER, duration_secs)
    }

    pub fn encode_notify_token(&self, identify: NotifyIdentify, duration_secs: u64) -> String {
        self.encode_token(identify, NOTIFY_ISSUER, duration_secs)
    }

    pub fn decode_call_token(&self, token: &str) -> Option<CallToken> {
        self.decode_token(token, CALL_ISSUER)
    }

    pub fn decode_notify_token(&self, token: &str) -> Option<NotifyIdentify> {
        self.decode_token(token, NOTIFY_ISSUER)
    }

    fn encode_token<T: Serialize + DeserializeOwned>(&self, token: T, issuer: &str, duration_secs: u64) -> String {
        let claims = Claims::with_custom_claims(token, Duration::from_secs(duration_secs)).with_issuer(issuer);
        self.key.authenticate(claims).expect("Should create jwt")
    }

    fn decode_token<T: Serialize + DeserializeOwned>(&self, token: &str, issuer: &str) -> Option<T> {
        let options = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[issuer])),
            ..Default::default()
        };
        let claims = self.key.verify_token::<T>(token, Some(options)).ok()?;
        if let Some(expires_at) = claims.expires_at {
            let now = Clock::now_since_epoch();
            if now >= expires_at {
                return None;
            }
        }
        Some(claims.custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CallDirection, InternalCallId};

    #[test]
    fn test_token_encoding_decoding() {
        let secret = "my_secret";
        let storage = AddressBookStorage::new(secret);
        let context = SecureContext::new(secret, storage);

        let call_token = CallToken {
            direction: CallDirection::Outgoing,
            call_id: InternalCallId::random(),
        };
        let encoded_token = context.encode_call_token(call_token.clone(), 100);
        let decoded_token = context.decode_call_token(&encoded_token).unwrap();
        assert_eq!(decoded_token, call_token);

        let notify_token = NotifyIdentify {
            app: "app1".to_owned().into(),
            client: "client1".to_owned(),
        };
        let encoded_token = context.encode_notify_token(notify_token.clone(), 100);
        let decoded_token = context.decode_notify_token(&encoded_token).unwrap();
        assert_eq!(decoded_token, notify_token);
    }

    #[test]
    fn test_token_expiration() {
        let secret = "my_secret";
        let storage = AddressBookStorage::new(secret);
        let context = SecureContext::new(secret, storage);

        let call_token = CallToken {
            direction: CallDirection::Outgoing,
            call_id: InternalCallId::random(),
        };
        let encoded_token = context.encode_call_token(call_token, 1);
        // Simulate expiration by waiting (or mocking the clock)
        std::thread::sleep(std::time::Duration::from_secs(2)); // Wait for token to expire
        assert_eq!(context.decode_call_token(&encoded_token), None);

        let notify_token = NotifyIdentify {
            app: "app1".to_owned().into(),
            client: "client1".to_owned(),
        };
        let encoded_token = context.encode_notify_token(notify_token, 1);
        // Simulate expiration by waiting (or mocking the clock)
        std::thread::sleep(std::time::Duration::from_secs(2)); // Wait for token to expire
        assert_eq!(context.decode_notify_token(&encoded_token), None);
    }
}
