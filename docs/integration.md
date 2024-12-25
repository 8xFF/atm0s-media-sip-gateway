# Integration Guide for SIP Gateway

## Overview

The SIP Gateway allows developers to manage SIP-based communication, including making outgoing calls and handling incoming calls. This guide provides detailed instructions on how to integrate with the SIP Gateway using its API and WebSocket interfaces.

## Outgoing Calls

### Steps

1. Phone numbers sync endpoint should contain number with outgoing SIP information or add a phone number in admin panel.
2. Create call by API (auth by app_secret, provide from number, to number)
3. Use SDK to show Outgoing UI with call_ws uri from step 1 (or manually implement it with websocket and media sdk)
4. Handling call event with hooks (included in phone number info)

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

1. Phone numbers sync endpoint should contain number with incoming SIP information or add a phone number in admin panel.
2. Create session_id token by API (auth by app_scret, provide session_id)
3. Use SDK to init SipIncomingHandler with notify_ws uri from step 2 (or manually implement with Websocket)
4. Show Incoming UI with SDK when received event from SipIncomingHandler (or manually implement it with websocket and media sdk)

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
