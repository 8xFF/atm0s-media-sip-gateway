# Integration Guide for SIP Gateway

## Overview

The SIP Gateway allows developers to manage SIP-based communication, including
making outgoing calls and handling incoming calls. This guide provides detailed
instructions on how to integrate with the SIP Gateway using its API and
WebSocket interfaces.

## Outgoing Calls

### Steps

1. Create call by API (auth by app_secret, provide from number, to number)
2. Use
   [SipOutgoing UI SDK](https://github.com/8xFF/atm0s-media-sdk-ts/tree/main/apps/web/app/react_ui_samples/sipOutgoing)
   to show Outgoing UI with call_ws uri from step 1 (or manually implement it
   with websocket and media sdk)
3. Control the call with APIs or direct from UI
4. Handling call event with hooks (included in phone number info)

Note that the call will be terminated when the websocket is closed.

### APIs

#### Create Outgoing Call

- **Endpoint**: POST `/call/outgoing`
- **Authentication**: Bearer Token
- **Request Body**:
  ```json
  {
    "sip_server": "string",
    "sip_proxy": "string",
    "sip_auth": {
      "username": "string",
      "password": "string"
    },
    "from_number": "string",
    "to_number": "string",
    "hook": "string",
    "hook_content_type": "Json" | "Protobuf",
    "streaming": {
      "room": "string",
      "peer": "string",
      "record": boolean
    }
  }
  ```

- **Response**:
  ```json
  {
    "status": true,
    "data": {
      "call_id": "string",
      "call_token": "string",
      "call_ws": "string"
    }
  }
  ```

#### End Outgoing Call

- **Endpoint**: DELETE `/call/{call_id}`
- **Authentication**: Bearer Token
- **Response**:
  ```json
  {
    "status": true
  }
  ```

## Incomings

### Steps

1. Register phone-number with Admin UI.
2. Handle incoming call event with hooks, use websocket or push notification to
   notify client about incoming call.
3. Show incoming call at client side, use can use
   [SipIncoming UI SDK](https://github.com/8xFF/atm0s-media-sdk-ts/tree/main/apps/web/app/react_ui_samples/sipIncoming)
   to init SipIncomingHandler with call_ws from step 2 (or manually implement
   with Websocket).
4. Control the call with APIs or direct from UI
5. Handling call event with hooks (included in phone number info)

Note that the call will be terminated when the websocket is closed.

### APIs

#### Manage Incoming Call

- **Endpoint**: POST `/call/incoming/{call_id}/action`
- **Authentication**: Bearer Token
- **Actions**:
  - `Ring`: Notify incoming call
  - `Accept`: Accept the call
  - `End`: Terminate the call
- **Request Body**:
  ```json
  {
    "action": "Ring" | "Accept" | "End",
    "stream": {
      "room": "string",
      "peer": "string",
      "record": boolean
    }
  }
  ```

#### Delete Call

- **Endpoint**: DELETE `/call/{call_id}`
- **Authentication**: Bearer Token
- **Response**:
  ```json
  {
    "status": true
  }
  ```
