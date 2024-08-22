import Srf, { Dialog, SrfRequest } from 'drachtio-srf'
import { Call, CallAction } from './lib'
import { Atm0sConfig, createAtm0sToken } from 'sip/atm0s'
import { rtpCreateOffer, rtpDelete, rtpSetAnswer } from 'sip/reqs'
import { EventEmitter } from 'stream'
import { feedbackStatus } from 'sip/hooks'
import { DRACHTIO_CONFIG } from 'config'

export class OutgoingCall extends EventEmitter implements Call {
  callId = 'out-' + new Date().getTime()
  req?: SrfRequest
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
    this.uac = await this.srf.createUAC(
      `sip:${this.to}@${this.dest}`,
      {
        localSdp: sdp,
        headers: {
          From: `sip:${this.from}@${DRACHTIO_CONFIG.sip_server}`,
        },
      },
      {
        cbRequest: (req) => {
          this.req = req
        },
      },
      async (err, uac) => {
        if (err) {
          console.log('[OutgoingCall] Outgoing error', err.status)
          await this.onRejected()
        } else {
          console.log('[OutgoingCall] Outgoing answer', uac.remote.sdp)
          uac.on('destroy', this.onEnded)
          await this.onAccepted(uac.remote.sdp)
        }
      },
    )
    console.log('[OutgoingCall] Created uac')
  }

  /** Internal handle */
  onAccepted = async (sdp: string) => {
    console.log('[OutgoingCall] OnAccept')
    this.emit('accepted')
    await rtpSetAnswer(this.rtpEndpoint!, sdp)
    await feedbackStatus(this.hook, { state: 'Accepted' })
  }

  onCanceled = async () => {
    console.log('[OutgoingCall] OnCanceled')
    this.emit('canceled')
    if (this.rtpEndpoint) {
      rtpDelete(this.rtpEndpoint)
    }
    if (this.req) {
      const callback = () => {
        return {}
      }
      this.req.cancel(callback)
    }
    await feedbackStatus(this.hook, { state: 'Canceled' })
  }

  onRejected = async () => {
    console.log('[OutgoingCall] OnRejected')
    this.emit('rejected')
    if (this.rtpEndpoint) {
      rtpDelete(this.rtpEndpoint)
    }
    await feedbackStatus(this.hook, { state: 'Rejected' })
  }

  onEnded = async () => {
    console.log('[OutgoingCall] OnDestroy')
    this.emit('ended')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
      delete this.rtpEndpoint
    }
    await feedbackStatus(this.hook, { state: 'Ended' })
  }

  /** Call interface: doAction method */
  async doAction(action: CallAction): Promise<void> {
    switch (action) {
      case 'Cancel': {
        await this.onCanceled()
        break
      }
      case 'End': {
        if (this.uac) {
          this.uac.destroy()
          delete this.uac
        } else {
          console.log(
            '[OutgoingCall] Call received End but not in runing state',
          )
        }
        break
      }
    }
  }
}
