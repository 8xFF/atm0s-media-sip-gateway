// This file is @generated by prost-build.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IncomingCallNotify {
    #[prost(string, tag = "1")]
    pub call_id: ::prost::alloc::string::String,
    #[prost(oneof = "incoming_call_notify::Event", tags = "10, 11, 12")]
    pub event: ::core::option::Option<incoming_call_notify::Event>,
}
/// Nested message and enum types in `IncomingCallNotify`.
pub mod incoming_call_notify {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct CallArrived {
        #[prost(string, tag = "1")]
        pub call_token: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub call_ws: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub from: ::prost::alloc::string::String,
        #[prost(string, tag = "4")]
        pub to: ::prost::alloc::string::String,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, Copy, PartialEq, ::prost::Message)]
    pub struct CallCancelled {}
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, Copy, PartialEq, ::prost::Message)]
    pub struct CallAccepted {}
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Event {
        #[prost(message, tag = "10")]
        Arrived(CallArrived),
        #[prost(message, tag = "11")]
        Cancelled(CallCancelled),
        #[prost(message, tag = "12")]
        Accepted(CallAccepted),
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IncomingCallData {
    #[prost(oneof = "incoming_call_data::Data", tags = "1, 2, 3")]
    pub data: ::core::option::Option<incoming_call_data::Data>,
}
/// Nested message and enum types in `IncomingCallData`.
pub mod incoming_call_data {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IncomingCallEvent {
        #[prost(oneof = "incoming_call_event::Event", tags = "1, 2, 3, 4")]
        pub event: ::core::option::Option<incoming_call_event::Event>,
    }
    /// Nested message and enum types in `IncomingCallEvent`.
    pub mod incoming_call_event {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct SipEvent {
            #[prost(oneof = "sip_event::Event", tags = "1, 2")]
            pub event: ::core::option::Option<sip_event::Event>,
        }
        /// Nested message and enum types in `SipEvent`.
        pub mod sip_event {
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Cancelled {}
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Bye {}
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Oneof)]
            pub enum Event {
                #[prost(message, tag = "1")]
                Cancelled(Cancelled),
                #[prost(message, tag = "2")]
                Bye(Bye),
            }
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Accepted {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Ended {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Error {
            #[prost(string, tag = "1")]
            pub message: ::prost::alloc::string::String,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Event {
            #[prost(message, tag = "1")]
            Err(Error),
            #[prost(message, tag = "2")]
            Sip(SipEvent),
            #[prost(message, tag = "3")]
            Accepted(Accepted),
            #[prost(message, tag = "4")]
            Ended(Ended),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IncomingCallRequest {
        #[prost(uint32, tag = "1")]
        pub req_id: u32,
        #[prost(oneof = "incoming_call_request::Action", tags = "10, 11, 12")]
        pub action: ::core::option::Option<incoming_call_request::Action>,
    }
    /// Nested message and enum types in `IncomingCallRequest`.
    pub mod incoming_call_request {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Ring {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Accept {
            #[prost(string, tag = "1")]
            pub room: ::prost::alloc::string::String,
            #[prost(string, tag = "2")]
            pub peer: ::prost::alloc::string::String,
            #[prost(bool, tag = "3")]
            pub record: bool,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct End {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Action {
            #[prost(message, tag = "10")]
            Ring(Ring),
            #[prost(message, tag = "11")]
            Accept(Accept),
            #[prost(message, tag = "12")]
            End(End),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IncomingCallResponse {
        #[prost(uint32, tag = "1")]
        pub req_id: u32,
        #[prost(oneof = "incoming_call_response::Response", tags = "10, 11, 12, 13")]
        pub response: ::core::option::Option<incoming_call_response::Response>,
    }
    /// Nested message and enum types in `IncomingCallResponse`.
    pub mod incoming_call_response {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Ring {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Accept {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct End {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Error {
            #[prost(string, tag = "1")]
            pub message: ::prost::alloc::string::String,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Response {
            #[prost(message, tag = "10")]
            Error(Error),
            #[prost(message, tag = "11")]
            Ring(Ring),
            #[prost(message, tag = "12")]
            Accept(Accept),
            #[prost(message, tag = "13")]
            End(End),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        Event(IncomingCallEvent),
        #[prost(message, tag = "2")]
        Request(IncomingCallRequest),
        #[prost(message, tag = "3")]
        Response(IncomingCallResponse),
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OutgoingCallData {
    #[prost(oneof = "outgoing_call_data::Data", tags = "1, 2, 3")]
    pub data: ::core::option::Option<outgoing_call_data::Data>,
}
/// Nested message and enum types in `OutgoingCallData`.
pub mod outgoing_call_data {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct OutgoingCallEvent {
        #[prost(oneof = "outgoing_call_event::Event", tags = "1, 2, 3")]
        pub event: ::core::option::Option<outgoing_call_event::Event>,
    }
    /// Nested message and enum types in `OutgoingCallEvent`.
    pub mod outgoing_call_event {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct SipEvent {
            #[prost(oneof = "sip_event::Event", tags = "1, 2, 3, 4, 5")]
            pub event: ::core::option::Option<sip_event::Event>,
        }
        /// Nested message and enum types in `SipEvent`.
        pub mod sip_event {
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Provisional {
                #[prost(uint32, tag = "1")]
                pub code: u32,
            }
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Early {
                #[prost(uint32, tag = "1")]
                pub code: u32,
            }
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Accepted {
                #[prost(uint32, tag = "1")]
                pub code: u32,
            }
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Failure {
                #[prost(uint32, tag = "1")]
                pub code: u32,
            }
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Message)]
            pub struct Bye {}
            #[derive(serde::Serialize, serde::Deserialize)]
            #[derive(Clone, Copy, PartialEq, ::prost::Oneof)]
            pub enum Event {
                #[prost(message, tag = "1")]
                Provisional(Provisional),
                #[prost(message, tag = "2")]
                Early(Early),
                #[prost(message, tag = "3")]
                Accepted(Accepted),
                #[prost(message, tag = "4")]
                Failure(Failure),
                #[prost(message, tag = "5")]
                Bye(Bye),
            }
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct Ended {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Error {
            #[prost(string, tag = "1")]
            pub message: ::prost::alloc::string::String,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Event {
            #[prost(message, tag = "1")]
            Err(Error),
            #[prost(message, tag = "2")]
            Sip(SipEvent),
            #[prost(message, tag = "3")]
            Ended(Ended),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, Copy, PartialEq, ::prost::Message)]
    pub struct OutgoingCallRequest {
        #[prost(uint32, tag = "1")]
        pub req_id: u32,
        #[prost(oneof = "outgoing_call_request::Action", tags = "10")]
        pub action: ::core::option::Option<outgoing_call_request::Action>,
    }
    /// Nested message and enum types in `OutgoingCallRequest`.
    pub mod outgoing_call_request {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct End {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Oneof)]
        pub enum Action {
            #[prost(message, tag = "10")]
            End(End),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct OutgoingCallResponse {
        #[prost(uint32, tag = "1")]
        pub req_id: u32,
        #[prost(oneof = "outgoing_call_response::Response", tags = "10, 11")]
        pub response: ::core::option::Option<outgoing_call_response::Response>,
    }
    /// Nested message and enum types in `OutgoingCallResponse`.
    pub mod outgoing_call_response {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Copy, PartialEq, ::prost::Message)]
        pub struct End {}
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Error {
            #[prost(string, tag = "1")]
            pub message: ::prost::alloc::string::String,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Response {
            #[prost(message, tag = "10")]
            Error(Error),
            #[prost(message, tag = "11")]
            End(End),
        }
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        Event(OutgoingCallEvent),
        #[prost(message, tag = "2")]
        Request(OutgoingCallRequest),
        #[prost(message, tag = "3")]
        Response(OutgoingCallResponse),
    }
}
