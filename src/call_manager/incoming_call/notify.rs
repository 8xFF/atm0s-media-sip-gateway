use atm0s_small_p2p::pubsub_service::{PubsubChannelId, PubsubServiceRequester};

use crate::{
    hook::HttpHookSenderNoContext,
    protocol::{
        protobuf::sip_gateway::{incoming_call_notify, IncomingCallNotify},
        HookIncomingCallRequest, IncomingCallActionRequest, InternalCallId, NotifyIdentify, PhoneNumber, PhoneNumberRoute,
    },
};

pub enum IncomingCallNotifySender {
    Http(HttpHookSenderNoContext, String),
    Websocket(PubsubServiceRequester, PubsubChannelId),
}

impl IncomingCallNotifySender {
    pub fn new(number: PhoneNumber, pubsub: PubsubServiceRequester, hook_sender: HttpHookSenderNoContext) -> Self {
        match number.route {
            PhoneNumberRoute::Static { client } => Self::Websocket(pubsub, NotifyIdentify { app: number.app_id.into(), client }.to_pubsub_channel()),
            PhoneNumberRoute::Dynamic { hook } => Self::Http(hook_sender, hook),
        }
    }

    pub async fn notify(&mut self, req: HookIncomingCallRequest) -> anyhow::Result<Option<IncomingCallActionRequest>> {
        match self {
            Self::Http(sender, endpoint) => Ok(Some(sender.request(&endpoint, &req).await?)),
            Self::Websocket(pubsub, channel) => {
                pubsub
                    .publish_as_guest_ob(
                        *channel,
                        IncomingCallNotify {
                            call_id: req.call_id.into(),
                            event: Some(incoming_call_notify::Event::Arrived(incoming_call_notify::CallArrived {
                                call_token: req.call_token,
                                call_ws: req.call_ws,
                                from: req.from,
                                to: req.to,
                            })),
                        },
                    )
                    .await?;
                Ok(None)
            }
        }
    }

    pub async fn cancel(&self, call_id: InternalCallId) -> anyhow::Result<()> {
        match self {
            Self::Http(..) => Ok(()),
            Self::Websocket(pubsub, channel) => {
                pubsub
                    .publish_as_guest_ob(
                        *channel,
                        IncomingCallNotify {
                            call_id: call_id.into(),
                            event: Some(incoming_call_notify::Event::Cancelled(incoming_call_notify::CallCancelled {})),
                        },
                    )
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn accept(&self, call_id: InternalCallId) -> anyhow::Result<()> {
        match self {
            Self::Http(..) => Ok(()),
            Self::Websocket(pubsub, channel) => {
                pubsub
                    .publish_as_guest_ob(
                        *channel,
                        IncomingCallNotify {
                            call_id: call_id.into(),
                            event: Some(incoming_call_notify::Event::Accepted(incoming_call_notify::CallAccepted {})),
                        },
                    )
                    .await?;
                Ok(())
            }
        }
    }
}
