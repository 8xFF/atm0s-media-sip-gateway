import { StreamingInfo } from 'schemes/make_call'

export type CallState =
  | 'Preparing'
  | 'Dialing'
  | 'Error'
  | 'Ringing'
  | 'Accepted'
  | 'Rejected'
  | 'Canceled'
  | 'Ended'

export interface IncallHookRequest {
  call_id: string
  sip_server: string
  from_number: string
  to_number: string
  ws: string
}

export interface IncallHookResponse {
  state: 'Ringing' | 'Canceled' | 'Accepted'
  hook: string
  streaming: StreamingInfo
}

export interface CallUpdateStatus {
  direction: 'in' | 'out'
  state: CallState
}

export interface AllowedNumber {
  sip_server: string
  number: string
}

export async function fetchPostJson<I, O>(url: string, body: I): Promise<O> {
  const res = await fetch(url, {
    headers: {
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
  )
  return response
}

export async function syncAllowedNumbers(
  url: string,
): Promise<AllowedNumber[]> {
  const res = await fetch(url)
  return res.json()
}
