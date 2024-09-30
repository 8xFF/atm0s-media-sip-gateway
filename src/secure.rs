use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::{CallDirection, InternalCallId};

const CALL_ISSUER: &str = "call";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct CallToken {
    pub direction: CallDirection,
    pub call_id: InternalCallId,
}

pub struct SecureContext {
    secret: String,
    key: HS256Key,
}

impl SecureContext {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_owned(),
            key: HS256Key::from_bytes(secret.as_bytes()),
        }
    }

    pub fn check_secret(&self, secret: &str) -> bool {
        self.secret.eq(secret)
    }

    pub fn encode_token(&self, token: CallToken, duration_secs: u64) -> String {
        let claims = Claims::with_custom_claims(token, Duration::from_secs(duration_secs)).with_issuer(CALL_ISSUER);
        self.key.authenticate(claims).expect("Should create jwt")
    }

    pub fn decode_token(&self, token: &str) -> Option<CallToken> {
        let options = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[CALL_ISSUER])),
            ..Default::default()
        };
        let claims = self.key.verify_token::<CallToken>(token, Some(options)).ok()?;
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
        let context = SecureContext::new(secret);

        let call_token = CallToken {
            direction: CallDirection::Outgoing,
            call_id: InternalCallId::random(),
        };

        let encoded_token = context.encode_token(call_token.clone(), 100);
        let decoded_token = context.decode_token(&encoded_token).unwrap();

        assert_eq!(decoded_token, call_token);
    }

    #[test]
    fn test_token_expiration() {
        let secret = "my_secret";
        let context = SecureContext::new(secret);

        let call_token = CallToken {
            direction: CallDirection::Outgoing,
            call_id: InternalCallId::random(),
        };

        let encoded_token = context.encode_token(call_token, 1);

        // Simulate expiration by waiting (or mocking the clock)
        std::thread::sleep(std::time::Duration::from_secs(2)); // Wait for token to expire

        assert_eq!(context.decode_token(&encoded_token), None);
    }
}
