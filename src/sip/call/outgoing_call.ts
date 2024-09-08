import Srf, { Dialog, SrfRequest, SrfResponse } from 'drachtio-srf'
import { Call, CallAction, CallActionResponse } from './lib'
import { rtpCreateOffer, rtpDelete, rtpSetAnswer } from 'sip/reqs'
import { EventEmitter } from 'events'
import { feedbackStatus } from 'sip/hooks'
import { DRACHTIO_CONFIG } from 'config'
import { StreamingInfo } from 'schemes/make_call'

export enum OutgoingCallEvent {
  StateChanged = 'StateChanged',
}

export enum OutgoingCallState {
  Preparing = 'Preparing',
  Dialing = 'Dialing',
  Ringing = 'Ringing',
  Rejected = 'Rejected',
  Error = 'Error',
  Canceled = 'Canceled',
  Accepted = 'Accepted',
  Ended = 'Ended',
}

export class OutgoingCall extends EventEmitter implements Call {
  callId = 'out-' + new Date().getTime()
  req?: SrfRequest
  res?: SrfResponse
  uac?: Dialog
  rtpEndpoint?: string

  constructor(
    private srf: Srf,
    private from: string,
    private to: string,
    private dest: string, // destination sip example: 123.123.123.123:5060
    private hook: string,
    private streaming: StreamingInfo,
  ) {
    super()
  }

  async makeCall() {
    this.fireEvent(OutgoingCallState.Preparing)
    const { endpoint, sdp } = await rtpCreateOffer(
      this.streaming.gateway,
      this.streaming.token,
    )
    this.rtpEndpoint = this.streaming.gateway + endpoint
    console.log('[OutgoingCall] create atm0s sdp', this.rtpEndpoint, sdp)
    this.fireEvent(OutgoingCallState.Dialing)
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
          if (res.status == 180) {
            this.fireEvent(OutgoingCallState.Ringing)
          }
          if (res.body) {
            // TODO how to handle early media
          }
        },
      } as any,
      (err, uac) => {
        this.uac = uac
        if (err) {
          console.log('[OutgoingCall] Outgoing error', err.status)

          // TODO handle for getting Cancel or Rejected or Error
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
    this.fireEvent(OutgoingCallState.Accepted)
  }

  onCanceled = async () => {
    console.log('[OutgoingCall] OnCanceled')
    this.emit('canceled')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Canceled)
  }

  onRejected = async () => {
    console.log('[OutgoingCall] OnRejected')
    this.emit('rejected')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Rejected)
  }

  onEnded = async () => {
    console.log('[OutgoingCall] OnDestroy')
    this.emit('ended')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Ended)
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
            error: 'WRONG_STATE',
            message: 'Cancel but this.req is not defined or already accepted',
          }
        }
      }
      case 'End': {
        if (this.uac) {
          this.uac.destroy()
          await this.onEnded()
          return { status: true, message: 'Ended' }
        } else {
          console.log(
            '[OutgoingCall] Call received End but not in accepted state',
          )
          return {
            status: false,
            error: 'WRONG_STATE',
            message: 'End but not in accepted state',
          }
        }
      }
      case 'ForceEnd': {
        if (this.uac) {
          this.uac.destroy()
          this.uac = undefined
          await this.onEnded()
          return { status: true, message: 'Ended' }
        } else if (this.req) {
          this.req.cancel(() => {
            return {}
          })
          this.req = undefined
          await this.onCanceled()
          return { status: true, message: 'Canceled' }
        } else {
          console.log(
            '[OutgoingCall] Call received ForceEnd but req and uac not defined',
          )
          return {
            status: false,
            error: 'WRONG_STATE',
            message: 'ForceEnd but req and uac not defined',
          }
        }
      }
    }
    return {
      status: false,
      error: 'UNSUPPORTED_ACTION',
      message: 'Unsupported action',
    }
  }

  fireEvent(state: OutgoingCallState) {
    this.emit(OutgoingCallEvent.StateChanged, state)
    feedbackStatus(this.hook, { state, direction: 'out' })
  }
}
