import Srf, { Dialog, SrfRequest, SrfResponse } from 'drachtio-srf'
import { Call, CallAction, CallActionResponse } from './lib'
import { Atm0sConfig, createAtm0sToken } from 'sip/atm0s'
import { rtpCreateOffer, rtpDelete, rtpSetAnswer } from 'sip/reqs'
import { EventEmitter } from 'stream'
import { feedbackStatus } from 'sip/hooks'
import { DRACHTIO_CONFIG } from 'config'

export class OutgoingCall extends EventEmitter implements Call {
  callId = 'out-' + new Date().getTime()
  req?: SrfRequest
  res?: SrfResponse
  uac?: Dialog
  rtpEndpoint?: string

  constructor(
    private srf: Srf,
    private atm0s: Atm0sConfig,
    private from: string,
    private to: string,
    private dest: string, // destination sip example: 123.123.123.123:5060
    private hook: string,
    private room: string,
    private peer: string,
  ) {
    super()
  }

  async makeCall() {
    const token = await createAtm0sToken(this.atm0s, this.room, this.peer)
    console.log('[OutgoingCall] create atm0s token', token)
    const { endpoint, sdp } = await rtpCreateOffer(this.atm0s.gateway, token)
    this.rtpEndpoint = this.atm0s.gateway + endpoint
    console.log('[OutgoingCall] create atm0s sdp', this.rtpEndpoint, sdp)
    this.srf.createUAC(
      `sip:${this.to}@${this.dest}`,
      {
        localSdp: sdp,
        headers: {
          From: `sip:${this.from}@${DRACHTIO_CONFIG.sip_server}`,
        },
      },
      {
        cbRequest: (err: any, req: SrfRequest) => {
          console.log('[OutgoingCall] req created', err)
          this.req = req
        },
        cbProvisional: (res: SrfResponse) => {
          console.log('[OutgoingCall] res provisional', res.status, res.body)
          if (res.body) {
            // TODO how to handle early media
          }
        },
      } as any,
      (err, uac) => {
        this.uac = uac
        if (err) {
          console.log('[OutgoingCall] Outgoing error', err.status)

          this.onRejected()
        } else {
          console.log(
            '[OutgoingCall] Outgoing answer',
            (uac as any).res?.status,
            uac.remote.sdp,
          )

          uac.on('destroy', () => {
            this.onEnded()
          })
          this.onAccepted(uac.remote.sdp)
        }
      },
    )
    console.log('[OutgoingCall] Created uac')
  }

  /** Internal handle, this function don't act with sip, only stream or fire event */
  onAccepted = async (sdp: string) => {
    console.log('[OutgoingCall] OnAccept')
    this.emit('accepted')
    await rtpSetAnswer(this.rtpEndpoint!, sdp)
    feedbackStatus(this.hook, { state: 'Accepted' })
  }

  onCanceled = async () => {
    console.log('[OutgoingCall] OnCanceled')
    this.emit('canceled')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    feedbackStatus(this.hook, { state: 'Canceled' })
  }

  onRejected = async () => {
    console.log('[OutgoingCall] OnRejected')
    this.emit('rejected')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    feedbackStatus(this.hook, { state: 'Rejected' })
  }

  onEnded = async () => {
    console.log('[OutgoingCall] OnDestroy')
    this.emit('ended')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    feedbackStatus(this.hook, { state: 'Ended' })
  }

  /** Call interface: doAction method */
  async doAction(action: CallAction): Promise<CallActionResponse> {
    switch (action) {
      case 'Cancel': {
        if (!this.uac && this.req) {
          this.req.cancel(() => {
            return {}
          })
          await this.onCanceled()
          return { status: true, message: 'Canceled' }
        } else {
          console.warn(
            '[OutgoingCall] cancel but this.req is not defined or already accepted',
          )
          return {
            status: false,
            message: 'Cancel but this.req is not defined or already accepted',
          }
        }
      }
      case 'End': {
        if (this.uac) {
          this.uac.destroy()
          return { status: true, message: 'Ended' }
        } else {
          console.log(
            '[OutgoingCall] Call received End but not in accepted state',
          )
          return {
            status: false,
            message: 'End but not in accepted state',
          }
        }
      }
    }
    return {
      status: false,
      message: 'Unsupported action',
    }
  }
}
