import Srf, { SrfRequest, SrfResponse, Dialog } from 'drachtio-srf'
import { Call, CallAction, CallActionResponse } from './lib'
import { feedbackStatus, IncallHookResponse } from 'sip/hooks'
import { rtpDelete, rtpCreateAnswer } from 'sip/reqs'
import { EventEmitter } from 'events'

export enum IncomingCallEvent {
  StateChanged = 'StateChanged',
}

export enum IncomingCallState {
  Ringing = 'Ringing',
  Rejected = 'Rejected',
  Error = 'Error',
  Canceled = 'Canceled',
  Accepted = 'Accepted',
  Ended = 'Ended',
}

export class IncomingCall extends EventEmitter implements Call {
  uas?: Dialog
  rtpEndpoint?: string

  constructor(
    private srf: Srf,
    private req: SrfRequest,
    private res: SrfResponse,
    private call: IncallHookResponse,
  ) {
    super()
    const req2 = req as any
    req2.on('cancel', () => {
      this.onCanceled()
    })
  }

  /** Internal handle */
  onCanceled = async () => {
    console.log('[IncomingCall] OnCanceled')
    this.fireEvent(IncomingCallState.Canceled)
  }

  onRejected = async () => {
    console.log('[IncomingCall] OnRejected')
    this.fireEvent(IncomingCallState.Rejected)
  }

  onEnded = async () => {
    console.log('[IncomingCall] OnEnded')
    this.emit('ended')
    if (this.rtpEndpoint) {
      await rtpDelete(this.rtpEndpoint)
      delete this.rtpEndpoint
    }
    this.fireEvent(IncomingCallState.Ended)
  }

  async onAccepted(): Promise<void> {
    const { endpoint, sdp } = await rtpCreateAnswer(
      this.call.streaming.gateway,
      this.req.body,
      this.call.streaming.token,
    )
    console.log('[IncomingCall] create atm0s sdp', endpoint, sdp)
    this.rtpEndpoint = this.call.streaming.gateway + endpoint
    this.uas = await this.srf.createUAS(this.req, this.res, {
      localSdp: sdp,
    })
    this.uas.on('destroy', () => {
      this.onEnded()
    })
    this.fireEvent(IncomingCallState.Accepted)
  }

  /** Call interface: doAction method */
  async doAction(action: CallAction): Promise<CallActionResponse> {
    switch (action) {
      case 'Accept': {
        if (!this.uas) {
          await this.onAccepted()
          return { status: true, message: 'Accepted' }
        } else {
          return {
            status: false,
            error: 'WRONG_STATE',
            message: 'Accept but already accepted state',
          }
        }
      }
      case 'Reject': {
        if (!this.uas) {
          await this.res.send(486) //Busy
          await this.onRejected()
          return { status: true, message: 'Rejected' }
        } else {
          return {
            status: false,
            error: 'WRONG_STATE',
            message: 'Reject but already accepted state',
          }
        }
      }
      case 'End': {
        if (this.uas) {
          this.uas.destroy()
          return { status: true, message: 'Ended' }
        } else {
          console.log(
            'Call received End but not in accepted state',
            this.req.callId,
          )
        }
        return {
          status: false,
          error: 'WRONG_STATE',
          message: 'End but not in accepted state',
        }
      }
      case 'ForceEnd': {
        if (this.uas) {
          this.uas.destroy()
          return { status: true, message: 'Ended' }
        } else {
          await this.res.send(486) //Busy
          await this.onRejected()
          return { status: true, message: 'Rejected' }
        }
      }
    }
    return {
      status: false,
      error: 'UNSUPPORTED_ACTION',
      message: 'Unsupported action',
    }
  }

  fireEvent(state: IncomingCallState) {
    this.emit(IncomingCallEvent.StateChanged, state)
    feedbackStatus(this.call.hook, { state, direction: 'in' })
  }
}
