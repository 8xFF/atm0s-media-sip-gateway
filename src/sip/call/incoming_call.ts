import Srf, { SrfRequest, SrfResponse, Dialog } from 'drachtio-srf'
import { Call, CallAction } from './lib'
import { feedbackStatus, IncallHookResponse } from 'sip/hooks'
import { Atm0sConfig, createAtm0sToken } from 'sip/atm0s'
import { rtpDelete, rtpCreateAnswer } from 'sip/reqs'
import { EventEmitter } from 'stream'

export type IncomingCallEvent = 'cancel' | 'end'

export class IncomingCall extends EventEmitter implements Call {
  uas?: Dialog
  rtpEndpoint?: string

  constructor(
    private srf: Srf,
    private atm0s: Atm0sConfig,
    private req: SrfRequest,
    private res: SrfResponse,
    private call: IncallHookResponse,
  ) {
    super()
    const req2 = req as any
    req2.on('cancel', this.onCancel)
  }

  /** Internal handle */
  onCancel = async () => {
    console.log('[IncomingCall] OnCanceled')
    this.emit('canceled')
    await feedbackStatus(this.call.hook, { state: 'Canceled' })
  }

  onRejected = async () => {
    console.log('[IncomingCall] OnRejected')
    this.emit('rejected')
    this.res.send(486) //Busy
    await feedbackStatus(this.call.hook, { state: 'Rejected' })
  }

  onEnd = async () => {
    console.log('[IncomingCall] OnEnded')
    this.emit('ended')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
      delete this.rtpEndpoint
    }
    await feedbackStatus(this.call.hook, { state: 'Ended' })
  }

  async onAccepted(): Promise<void> {
    const atm0s_token = await createAtm0sToken(
      this.atm0s,
      this.call.room_id,
      this.call.peer_id,
    )
    console.log('[IncomingCall] create atm0s token', atm0s_token)
    const { endpoint, sdp } = await rtpCreateAnswer(
      this.atm0s.gateway,
      this.req.body,
      atm0s_token,
    )
    console.log('[IncomingCall] create atm0s sdp', endpoint, sdp)
    this.rtpEndpoint = this.atm0s.gateway + endpoint
    this.uas = await this.srf.createUAS(this.req, this.res, {
      localSdp: sdp,
    })
    this.uas.on('destroy', this.onEnd)
    await feedbackStatus(this.call.hook, { state: 'Accepted' })
  }

  /** Call interface: doAction method */
  async doAction(action: CallAction): Promise<void> {
    switch (action) {
      case 'Accept': {
        await this.onAccepted()
        break
      }
      case 'Cancel': {
        await this.onCancel()
        break
      }
      case 'Reject': {
        await this.onRejected()
        break
      }
      case 'End': {
        if (this.uas) {
          this.uas.destroy()
          delete this.uas
        } else {
          console.log(
            'Call received End but not in runing state',
            this.req.callId,
          )
        }
        break
      }
    }
  }
}
