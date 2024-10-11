use std::sync::Arc;

use poem_openapi::{payload::Json, OpenApi};

use crate::{
    protocol::{CreateNotifyTokenRequest, CreateNotifyTokenResponse, TokenApiError},
    secure::{NotifyToken, SecureContext},
};

use super::{header_secret::TokenAuthorization, response_result::ApiRes};

pub struct TokenApis {
    pub secure_ctx: Arc<SecureContext>,
}

#[OpenApi]
impl TokenApis {
    #[oai(path = "/notify", method = "post")]
    async fn create_notify(&self, secret: TokenAuthorization, data: Json<CreateNotifyTokenRequest>) -> ApiRes<CreateNotifyTokenResponse, TokenApiError> {
        // TODO dynamic with apps secret
        if !self.secure_ctx.check_secret(&secret.0.token) {
            return Err(TokenApiError::WrongSecret.into());
        }

        let token = NotifyToken {
            app: "".to_string(),
            client: data.0.client_id,
        };
        let token = self.secure_ctx.encode_notify_token(token, data.0.ttl);

        Ok(CreateNotifyTokenResponse { token }.into())
    }
}
