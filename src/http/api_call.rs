use std::{sync::Arc, time::Duration};

use atm0s_small_p2p::pubsub_service::PubsubServiceRequester;
use poem::web::Query;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::{
    protocol::{
        protobuf::sip_gateway::{
            incoming_call_data::{incoming_call_request, incoming_call_response},
            outgoing_call_data::{outgoing_call_request, outgoing_call_response},
        },
        CallApiError, CreateCallRequest, CreateCallResponse, IncomingCallActionRequest, IncomingCallActionResponse, OutgoingCallActionRequest, OutgoingCallActionResponse,
    },
    secure::SecureContext,
    sip::MediaApi,
};

use super::{header_secret::TokenAuthorization, response_result::ApiRes, HttpCommand};

const RPC_TIMEOUT_SECONDS: u64 = 2;

pub struct CallApis {
    pub media_gateway: String,
    pub secure_ctx: Arc<SecureContext>,
    pub tx: Sender<HttpCommand>,
    pub call_pubsub: PubsubServiceRequester,
}

#[OpenApi]
impl CallApis {
    #[oai(path = "/outgoing", method = "post")]
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

    #[oai(path = "/outgoing/:call_id/action", method = "post")]
    async fn action_outcall(&self, Path(call_id): Path<String>, Query(token): Query<String>, data: Json<OutgoingCallActionRequest>) -> ApiRes<OutgoingCallActionResponse, CallApiError> {
        let token = if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
            token
        } else {
            return Err(CallApiError::WrongToken.into());
        };

        let channel = token.call_id.to_pubsub_channel();
        let req: outgoing_call_request::Action = data.0.try_into().map_err(|e| CallApiError::BadRequest(e))?;
        let res = self
            .call_pubsub
            .feedback_rpc_as_guest_ob::<_, outgoing_call_response::Response>(channel, "action", &req, Duration::from_secs(RPC_TIMEOUT_SECONDS))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;
        let res: OutgoingCallActionResponse = res.try_into().map_err(|e| CallApiError::SipError(e))?;
        Ok(res.into())
    }

    #[oai(path = "/incoming/:call_id/action", method = "post")]
    async fn action_incall(&self, Path(call_id): Path<String>, Query(token): Query<String>, data: Json<IncomingCallActionRequest>) -> ApiRes<IncomingCallActionResponse, CallApiError> {
        let token = if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
            token
        } else {
            return Err(CallApiError::WrongToken.into());
        };

        let channel = token.call_id.to_pubsub_channel();
        let req: incoming_call_request::Action = data.0.try_into().map_err(|e| CallApiError::BadRequest(e))?;
        let res = self
            .call_pubsub
            .feedback_rpc_as_guest_ob::<_, incoming_call_response::Response>(channel, "action", &req, Duration::from_secs(RPC_TIMEOUT_SECONDS))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;
        let res: IncomingCallActionResponse = res.try_into().map_err(|e| CallApiError::SipError(e))?;
        Ok(res.into())
    }

    #[oai(path = "/outgoing/:call_id", method = "delete")]
    async fn end_outgoing_call(&self, Query(token): Query<String>, Path(call_id): Path<String>) -> ApiRes<String, CallApiError> {
        let token = if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
            token
        } else {
            return Err(CallApiError::WrongToken.into());
        };

        let channel = token.call_id.to_pubsub_channel();
        let req = outgoing_call_request::Action::End(Default::default());
        let res = self
            .call_pubsub
            .feedback_rpc_as_guest_ob::<_, outgoing_call_response::Response>(channel, "destroy", &req, Duration::from_secs(RPC_TIMEOUT_SECONDS))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;
        let _: OutgoingCallActionResponse = res.try_into().map_err(|e| CallApiError::SipError(e))?;
        Ok("OK".to_owned().into())
    }

    #[oai(path = "/incoming/:call_id", method = "delete")]
    async fn end_incoming_call(&self, Query(token): Query<String>, Path(call_id): Path<String>) -> ApiRes<String, CallApiError> {
        let token = if let Some(token) = self.secure_ctx.decode_call_token(&token) {
            if *token.call_id != call_id {
                return Err(CallApiError::WrongToken.into());
            }
            token
        } else {
            return Err(CallApiError::WrongToken.into());
        };

        let channel = token.call_id.to_pubsub_channel();
        let req = incoming_call_request::Action::End(Default::default());
        let res = self
            .call_pubsub
            .feedback_rpc_as_guest_ob::<_, incoming_call_response::Response>(channel, "destroy", &req, Duration::from_secs(RPC_TIMEOUT_SECONDS))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;
        let _: IncomingCallActionResponse = res.try_into().map_err(|e| CallApiError::SipError(e))?;
        Ok("OK".to_owned().into())
    }
}
