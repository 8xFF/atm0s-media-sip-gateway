import { createAtm0sToken } from 'atm0s'
import { RTP_ENGINE_CONFIG } from 'config'
import { parseUri, SrfRequest, SrfResponse } from 'drachtio-srf'
import { srf, STORAGE } from 'index'
const RtpEngine = require('rtpengine-client').Client

const rtpengine = new RtpEngine()

async function rtpOffer(
  call: string,
  from: string,
  to: string | undefined,
  sdp: string,
  token: string,
): Promise<{ conn: string; sdp: string }> {
  const res = await rtpengine.offer(
    RTP_ENGINE_CONFIG.port,
    RTP_ENGINE_CONFIG.host,
    {
      'call-id': call,
      'from-tag': from,
      'to-tag': to,
      sdp,
      'atm0s-token': token,
    },
  )
  if (!res.conn || !res.sdp) {
    throw 'CANNOT_CREATE_SDP'
  }
  return res
}

async function rtpDelete(
  call: string,
  from: string,
  to: string | undefined,
  conn: string,
) {
  await rtpengine.delete(RTP_ENGINE_CONFIG.port, RTP_ENGINE_CONFIG.host, {
    'call-id': call,
    'from-tag': from,
    'to-tag': to,
    'conn-id': conn,
  })
}

export async function handleInvite(req: SrfRequest, res: SrfResponse) {
  const call_id = req.headers['call-id']
  const from = req.getParsedHeader('from')
  const to = req.getParsedHeader('to')

  const room = call_id
  const from_peer = parseUri(from.uri).user
  const to_peer = parseUri(to.uri).user

  const dests = await STORAGE.getUserDests(to_peer)
  if (dests.length == 0) {
    res.send(486, 'So sorry, busy right now', {})
    return
  }
  const dest = dests[dests.length - 1]

  console.log(`Incoming call ${call_id} from ${from.uri} to ${to.uri}`)
  const from_token = await createAtm0sToken(room, from_peer)
  const to_token = await createAtm0sToken(room, to_peer)
  const from_client_sdp = req.body
  const from_atm0s = await rtpOffer(
    call_id,
    from.params!.tag,
    to.params?.tag,
    from_client_sdp,
    from_token,
  )
  let to_atm0s_conn: string | null = null

  const localSdpA = async (to_client_sdp: string) => {
    const to_atm0s = await rtpOffer(
      call_id,
      from.params!.tag,
      to.params!.tag,
      to_client_sdp,
      to_token,
    )
    to_atm0s_conn = to_atm0s.conn
    return to_atm0s.sdp
  }

  try {
    console.log(
      `Incoming call ${call_id} creating UAC and UAS with dest ${dest}`,
    )
    const { uac, uas } = await srf.createB2BUA(req, res, dest, {
      localSdpB: from_atm0s.sdp,
      localSdpA,
    })
    console.log(
      `Incoming call ${call_id} created UAC and UAS with dest ${dest}`,
    )

    uac.on('destroy', async () => {
      uas.destroy()
      console.log(`Incoming call ${call_id} on destroy UAC`)
      console.log(`Deleting atm0s conns [${from_atm0s.conn}, ${to_atm0s_conn}]`)
      await rtpDelete(
        call_id,
        from.params!.tag,
        to.params!.tag,
        from_atm0s.conn,
      )
      await rtpDelete(call_id, from.params!.tag, to.params!.tag, to_atm0s_conn!)
      console.log(`Deleted atm0s conns [${from_atm0s.conn}, ${to_atm0s_conn}]`)
    })
    uas.on('destroy', async () => {
      uac.destroy()
      console.log(`Incoming call ${call_id} on destroy UAS`)
      console.log(`Deleting atm0s conns [${from_atm0s.conn}, ${to_atm0s_conn}]`)
      await rtpDelete(
        call_id,
        from.params!.tag,
        to.params!.tag,
        from_atm0s.conn,
      )
      await rtpDelete(call_id, from.params!.tag, to.params!.tag, to_atm0s_conn!)
      console.log(`Deleted atm0s conns [${from_atm0s.conn}, ${to_atm0s_conn}]`)
    })
  } catch (err) {
    console.log(`Incoming call ${call_id} create UAC and UAS error ${err}`)
    console.log(`Deleting atm0s conns [${from_atm0s.conn}]`)
    await rtpDelete(call_id, from.params!.tag, to.params!.tag, from_atm0s.conn)
    console.log(`Deleted atm0s conns [${from_atm0s.conn}]`)
  }
}
