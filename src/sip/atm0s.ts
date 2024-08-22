interface CreateTokenRes {
  status: boolean
  error: string
  data?: {
    token: string
  }
}

export interface Atm0sConfig {
  gateway: string
  secret: string
}

export async function createAtm0sToken(
  config: Atm0sConfig,
  room: string,
  peer: string,
) {
  const res = await fetch(config.gateway + '/token/rtpengine', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + config.secret,
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
