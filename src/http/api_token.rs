use std::sync::Arc;

use poem_openapi::{payload::Json, OpenApi};

use crate::{
    protocol::{CreateNotifyTokenRequest, CreateNotifyTokenResponse, NotifyIdentify, TokenApiError},
    secure::SecureContext,
};

use super::{header_secret::TokenAuthorization, response_result::ApiRes};

pub struct TokenApis {
    pub secure_ctx: Arc<SecureContext>,
}

#[OpenApi]
impl TokenApis {
    #[oai(path = "/notify", method = "post")]
    async fn create_notify(&self, secret: TokenAuthorization, data: Json<CreateNotifyTokenRequest>) -> ApiRes<CreateNotifyTokenResponse, TokenApiError> {
        let app_id = self.secure_ctx.check_secret(&secret.0.token).ok_or::<TokenApiError>(TokenApiError::WrongSecret.into())?;

        let identify = NotifyIdentify {
            app: app_id.into(),
            client: data.0.client_id,
        };
        let token = self.secure_ctx.encode_notify_token(identify, data.0.ttl);

        Ok(CreateNotifyTokenResponse { token }.into())
    }
}
