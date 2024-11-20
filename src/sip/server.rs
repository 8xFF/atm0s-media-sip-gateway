use std::{
    io,
    net::{IpAddr, SocketAddr},
};

use ezk_sip_core::{transport::udp::Udp, Endpoint, LayerKey};
use ezk_sip_types::{
    header::typed::Contact,
    uri::{sip::SipUri, NameAddr},
};
use ezk_sip_ua::{dialog::DialogLayer, invite::InviteLayer};
use incoming::InviteAcceptLayer;
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver};

use crate::protocol::{SipAuth, StreamingInfo};

mod incoming;
mod outgoing;

pub use incoming::{SipIncomingCall, SipIncomingCallOut};
pub use outgoing::{SipOutgoingCall, SipOutgoingCallError, SipOutgoingCallOut};

use super::MediaApi;

#[derive(Debug, Error)]
pub enum SipServerError {
    #[error("Unknown error")]
    Unknown,
}

pub enum SipServerOut {
    Incoming(SipIncomingCall),
}

pub struct SipServer {
    endpoint: Endpoint,
    contact: Contact,
    dialog_layer: LayerKey<DialogLayer>,
    invite_layer: LayerKey<InviteLayer>,
    incoming_rx: Receiver<SipIncomingCall>,
}

impl SipServer {
    pub async fn new(mut addr: SocketAddr, public_ip: IpAddr) -> io::Result<Self> {
        log::warn!("[SipServer] force set sip bind ip to {public_ip}. TODO: need to allow nat-traversal");
        addr.set_ip(public_ip);

        let mut builder = Endpoint::builder();

        let dialog_layer = builder.add_layer(DialogLayer::default());
        let invite_layer = builder.add_layer(InviteLayer::default());

        let contact: SipUri = format!("sip:atm0s@{}:{}", public_ip, addr.port()).parse().expect("Should parse");
        let contact = Contact::new(NameAddr::uri(contact));

        let (incoming_tx, incoming_rx) = channel(10);
        builder.add_layer(InviteAcceptLayer::new(incoming_tx, contact.clone(), dialog_layer, invite_layer));

        Udp::spawn(&mut builder, addr).await?;

        // Build endpoint to start the SIP Stack
        let endpoint = builder.build();

        Ok(Self {
            endpoint,
            contact,
            dialog_layer,
            invite_layer,
            incoming_rx,
        })
    }

    pub fn make_call(&self, media_api: MediaApi, from: &str, to: &str, auth: Option<SipAuth>, stream: StreamingInfo) -> Result<SipOutgoingCall, SipOutgoingCallError> {
        SipOutgoingCall::new(media_api, self.endpoint.clone(), self.dialog_layer, self.invite_layer, from, to, self.contact.clone(), auth, stream)
    }

    pub async fn recv(&mut self) -> Option<SipServerOut> {
        self.incoming_rx.recv().await.map(SipServerOut::Incoming)
    }
}
