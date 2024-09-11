import Srf, { Dialog, SrfRequest, SrfResponse } from 'drachtio-srf'
import { Call, CallAction, CallActionResponse } from './lib'
import { rtpCreateOffer, rtpDelete, rtpSetAnswer } from 'sip/reqs'
import { EventEmitter } from 'events'
import { feedbackStatus } from 'sip/hooks'
import { SipAuth, StreamingInfo } from 'schemes/make_call'

export enum OutgoingCallEvent {
  StateChanged = 'StateChanged',
}

export enum OutgoingCallState {
  Preparing = 'Preparing',
  Connecting = 'Connecting',
  Provisioning = 'Provisioning',
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
    private sip_server: string, // destination sip example: 123.123.123.123:5060
    private sip_auth: SipAuth | undefined,
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
    this.fireEvent(OutgoingCallState.Connecting)
    this.srf.createUAC(
      `sip:${this.to}@${this.sip_server}`,
      {
        localSdp: sdp,
        headers: {
          From: `sip:${this.from}@${this.sip_server}`,
        },
        auth: this.sip_auth,
      },
      {
        cbRequest: (err: any, req: SrfRequest) => {
          console.log('[OutgoingCall] req created', err)
          this.req = req
        },
        cbProvisional: (res: SrfResponse) => {
          console.log('[OutgoingCall] res provisional', res.status, res.body)
          this.fireEvent(OutgoingCallState.Provisioning, res.status)
          if (res.body) {
            // TODO how to handle early media
          }
        },
      } as any,
      (err, uac) => {
        this.uac = uac
        if (err) {
          console.log('[OutgoingCall] Outgoing error', err.status)
          this.onError(err.status)
        } else {
          console.log(
            '[OutgoingCall] Outgoing answer',
            (uac as any).res?.status,
            uac.remote.sdp,
          )

          uac.on('destroy', () => {
            this.onEnded()
          })
          this.onAccepted(uac.remote.sdp, (uac as any).res?.status)
        }
      },
    )
    console.log('[OutgoingCall] Created uac')
  }

  /** Internal handle, this function don't act with sip, only stream or fire event */
  onAccepted = async (sdp: string, code?: number) => {
    console.log('[OutgoingCall] OnAccept', code)
    await rtpSetAnswer(this.rtpEndpoint!, sdp)
    this.fireEvent(OutgoingCallState.Accepted, code)
  }

  onCanceled = async () => {
    console.log('[OutgoingCall] OnCanceled')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Canceled)
  }

  onError = async (code: number) => {
    console.log('[OutgoingCall] OnError', code)
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Error, code)
  }

  onEnded = async (code?: number) => {
    console.log('[OutgoingCall] OnDestroy')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
    }
    this.fireEvent(OutgoingCallState.Ended, code)
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

  fireEvent(state: OutgoingCallState, code?: number) {
    this.emit(OutgoingCallEvent.StateChanged, { state, direction: 'out', code })
    feedbackStatus(this.hook, { state, direction: 'out', code })
  }
}
