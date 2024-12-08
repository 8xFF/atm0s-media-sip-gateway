# SIP Gateway Hooks

Each hook supports two payload formats: Protobuf JSON or Protobuf Binary.

## Incoming Notify Hooks

These hooks handle incoming call event notifications such as call arrival,
cancellation, rejection, or acceptance. They are designed to trigger incoming
call UI updates through WebSocket or mobile push notifications.

When a third-party server receives a call event, it should respond with an
`IncomingCallResponse` message. The response format (JSON or Protobuf) is
determined by the phone number configuration.

## Incoming Call Hooks

These hooks handle incoming call status updates. The endpoint for these hooks is
configured in the phone number settings.

## Outgoing Call Hooks

These hooks handle outgoing call status updates. The endpoint for these hooks is
configured when create outgoing call.

## Protobuf Schema

For detailed message definitions, refer to the
[SipGateway](/protobuf/sip_gateway.proto) schema.
