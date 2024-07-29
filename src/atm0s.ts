import { ATM0S_CONFIG } from 'config'

interface CreateTokenRes {
  status: boolean
  error: string
  data?: {
    token: string
  }
}

export async function createAtm0sToken(room: string, peer: string) {
  const res = await fetch(ATM0S_CONFIG.GATEWAY + '/token/rtpengine', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + ATM0S_CONFIG.SECRET,
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      room: room,
      peer: peer,
      ttl: 10000,
    }),
  })
  const res_json = (await res.json()) as CreateTokenRes
  if (res_json.data?.token) {
    return res_json.data?.token
  } else {
    throw 'CREATE_TOKEN_ERROR'
  }
}
