use std::sync::Arc;

use poem::web::Query;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    protocol::{CallActionRequest, CallActionResponse, CallApiError, CreateCallRequest, CreateCallResponse},
    secure::SecureContext,
    sip::MediaApi,
};

use super::{header_secret::TokenAuthorization, response_result::ApiRes, HttpCommand};

pub struct CallApis {
    pub media_gateway: String,
    pub secure_ctx: Arc<SecureContext>,
    pub tx: Sender<HttpCommand>,
}

#[OpenApi]
impl CallApis {
    #[oai(path = "/", method = "post")]
    async fn create_call(&self, secret: TokenAuthorization, data: Json<CreateCallRequest>) -> ApiRes<CreateCallResponse, CallApiError> {
        // TODO dynamic with apps secret
        if !self.secure_ctx.check_secret(&secret.0.token) {
            return Err(CallApiError::WrongSecret.into());
        }
        let media_api = MediaApi::new(&self.media_gateway, &secret.0.token);

        let (tx, rx) = oneshot::channel();
        self.tx
            .send(HttpCommand::CreateCall(data.0, media_api, tx))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        let res = rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))??;
        Ok(res.into())
    }

    #[oai(path = "/:call_id/action", method = "post")]
    async fn action_call(&self, Path(call_id): Path<String>, Query(token): Query<String>, data: Json<CallActionRequest>) -> ApiRes<CallActionResponse, CallApiError> {
        if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
        } else {
            return Err(CallApiError::WrongToken.into());
        }

        let (tx, rx) = oneshot::channel();
        self.tx
            .send(HttpCommand::ActionCall(call_id.into(), data.0, tx))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))?.map_err(|e| CallApiError::SipError(e.to_string()))?;
        Ok(CallActionResponse {}.into())
    }

    #[oai(path = "/:call_id", method = "delete")]
    async fn encode_call(&self, Query(token): Query<String>, Path(call_id): Path<String>) -> ApiRes<String, CallApiError> {
        if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
        } else {
            return Err(CallApiError::WrongToken.into());
        }

        let (tx, rx) = oneshot::channel();
        self.tx.send(HttpCommand::EndCall(call_id.into(), tx)).await.map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))??;
        Ok("OK".to_string().into())
    }
}
