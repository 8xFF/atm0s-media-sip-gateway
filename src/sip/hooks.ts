import { StreamingInfo } from 'schemes/make_call'
import { OutgoingCallState } from './call/outgoing_call'
import { IncomingCallState } from './call/incoming_call'
import { SECRET } from 'config'

export type CallState = OutgoingCallState | IncomingCallState

export interface IncallHookRequest {
  call_id: string
  sip_server: string
  from_number: string
  to_number: string
  ws: string
}

export interface IncallHookResponse {
  state: 'Ringing' | 'Rejected' | 'Accepted'
  hook?: string
  streaming?: StreamingInfo
}

export interface CallUpdateStatus {
  direction: 'in' | 'out'
  state: CallState
  code?: number
}

export interface AllowedNumber {
  subnet: string
  number: string
}

export async function fetchPostJson<I, O>(
  url: string,
  body: I,
  headers?: any,
): Promise<O> {
  const res = await fetch(url, {
    headers: {
      ...headers,
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    method: 'POST',
    body: JSON.stringify(body),
  })

  if (res.status == 200) {
    return res.json() as O
  } else {
    throw new Error('HookError')
  }
}

export async function feedbackStatus(
  url: string,
  body: CallUpdateStatus,
): Promise<[string | null, any | null]> {
  try {
    const res = await fetch(url, {
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      method: 'POST',
      body: JSON.stringify(body),
    })

    if (res.status == 200) {
      const text = await res.text()
      return [text, null]
    } else {
      console.error('Feedback status error', url, res.statusText)
      return [null, res.statusText]
    }
  } catch (e) {
    console.error('Feedback status error', url, e)
    return [null, e]
  }
}

export async function hookIncoming(
  url: string,
  sip_server: string,
  call_id: string,
  from_number: string,
  to_number: string,
): Promise<IncallHookResponse> {
  const response = await fetchPostJson<IncallHookRequest, IncallHookResponse>(
    url,
    {
      sip_server,
      call_id,
      from_number,
      to_number,
      ws: '/ws/call/' + call_id,
    },
    { 'X-API-Key': SECRET },
  )
  return response
}

export async function syncAllowedNumbers(
  url: string,
): Promise<AllowedNumber[]> {
  try {
    const res = await fetch(url, {
      headers: {
        'X-API-Key': SECRET,
      },
    })
    const res_json = await res.json()
    if (res_json.status) {
      return res_json.data
    } else {
      console.log('sync allowed numbers error', res_json.error)
      return []
    }
  } catch (e) {
    console.error('sync allowed numbers error', e)
    return []
  }
}
