# Integration Guide for SIP Gateway

## Overview

The SIP Gateway allows developers to manage SIP-based communication, including making outgoing calls and handling incoming calls. This guide provides detailed instructions on how to integrate with the SIP Gateway using its API and WebSocket interfaces.

## Outgoing Calls

### Steps

1. Phone numbers sync endpoint should contain number with outgoing SIP information or add a phone number in admin panel.
2. Create call by API (auth by app_secret, provide from number, to number)
3. Use SDK to show Outgoing UI with call_ws uri from step 1 (or manualy implement it with websocket and media sdk)

4. Handling call event with hooks (included in phone number info)

## Incomings

### Steps

1. Phone numbers sync endpoint should contain number with incoming SIP information or add a phone number in admin panel.
2. Create session_id token by API (auth by app_scret, provide session_id)
3. Use SDK to init SipIncomingHandler with notify_ws uri from step 2 (or manualy implement with Websocket)
4. Show Incoming UI with SDK when received event from SipIncomingHandler (or manualy implement it with websocket and media sdk)
