import Srf, { SrfConfig, SrfRequest, SrfResponse } from 'drachtio-srf'
import {
  CallUpdateStatus,
  feedbackStatus,
  hookIncoming,
  syncAllowedNumbers,
} from './hooks'
import {
  IncomingCall,
  IncomingCallEvent,
  IncomingCallState,
} from './call/incoming_call'
import {
  OutgoingCall,
  OutgoingCallEvent,
  OutgoingCallState,
} from './call/outgoing_call'
import { CallAction } from './call/lib'
import EventEmitter from 'events'
import { SipAuth, StreamingInfo } from 'schemes/make_call'
import { Netmask } from 'netmask'

export enum SipCallEvent {
  StateChanged = 'StateChanged',
}

export class SipGateway extends EventEmitter {
  srf = new Srf()
  incoming_calls: Map<string, IncomingCall> = new Map()
  outgoing_calls: Map<string, OutgoingCall> = new Map()
  allowed_numbers: Map<string, string> = new Map()

  constructor(
    private config: SrfConfig,
    private incoming_hook: string,
    private allowed_numbers_sync: string | undefined,
  ) {
    super()
    this.srf
      .on('connect', (err, hostPort) => {
        if (!err) {
          console.log(`Connected to drachtio ${hostPort} success`)
        } else {
          console.log(`Connect to drachtio ${hostPort} error: `, err)
        }
      })
      .on('error', (err) => {
        console.log(`Srf error: ${err}`)
      })

    this.srf.invite(async (req, res) => {
      try {
        return await this.onInvite(req, res)
      } catch (err) {
        console.error('handle invite error', err)
      }
    })

    if (this.allowed_numbers_sync) {
      this.syncAllowed()
      setInterval(this.syncAllowed, 60000)
    }
  }

  connect() {
    return this.srf.connect(this.config)
  }

  syncAllowed = async () => {
    const numbers = await syncAllowedNumbers(this.allowed_numbers_sync!)
    this.allowed_numbers.clear()
    numbers.forEach((allowed) => {
      this.allowed_numbers.set(allowed.number, allowed.subnet)
    })
  }

  async callAction(call_id: string, action: CallAction) {
    const call =
      this.incoming_calls.get(call_id) || this.outgoing_calls.get(call_id)
    if (call) {
      return await call.doAction(action)
    } else {
      return {
        status: false,
        error: 'CALL_NOT_FOUND',
        message: 'Provided call_id not found',
      }
    }
  }

  async makeCall(
    sip_server: string,
    sip_auth: SipAuth | undefined,
    from_number: string,
    to_number: string,
    status_hook: string,
    streaming: StreamingInfo,
  ) {
    const call_id = 'out-' + new Date().getTime()
    const outgoing_call = new OutgoingCall(
      this.srf,
      from_number,
      to_number,
      sip_server,
      sip_auth,
      status_hook,
      streaming,
    )
    await outgoing_call.makeCall()
    outgoing_call.on(
      OutgoingCallEvent.StateChanged,
      (status: CallUpdateStatus) => {
        this.emit(SipCallEvent.StateChanged, [call_id, status])
        switch (status.state) {
          case OutgoingCallState.Canceled:
          case OutgoingCallState.Error:
          case OutgoingCallState.Ended:
            this.outgoing_calls.delete(call_id)
            break
        }
      },
    )
    this.outgoing_calls.set(call_id, outgoing_call)

    return call_id
  }

  /** handle from srf callback */
  private onInvite = async (req: SrfRequest, res: SrfResponse) => {
    const call_id = req.headers['call-id']
    const from = req.getParsedHeader('from')
    const to = req.getParsedHeader('to')
    const sip_server = req.source_address + ':' + req.source_port

    const from_peer = (Srf as any).parseUri(from.uri).user
    const to_peer = (Srf as any).parseUri(to.uri).user
    const allowed_subnet = this.allowed_numbers.get(to_peer)

    if (
      allowed_subnet &&
      new Netmask(allowed_subnet).contains(req.source_address)
    ) {
      console.log('Call from', sip_server, call_id, from_peer, to_peer)
    } else {
      console.warn(
        'Call from untrusted source',
        allowed_subnet,
        sip_server,
        to_peer,
      )
      res.send(406) //trying
      return
    }

    res.send(100) //trying
    const req2 = req as any
    let canceled = false
    const handle_cancel = () => {
      canceled = true
    }
    req2.on('cancel', handle_cancel)

    try {
      const response = await hookIncoming(
        this.incoming_hook,
        sip_server,
        call_id,
        from_peer,
        to_peer,
      )
      req2.off('cancel', handle_cancel)
      console.log('Call response', call_id, response)

      if (canceled && response.hook) {
        console.log('Call canceled from caller', call_id)
        feedbackStatus(response.hook, {
          state: IncomingCallState.Canceled,
          direction: 'in',
        })
        return
      }

      switch (response.state) {
        case 'Accepted': {
          const call = new IncomingCall(
            this.srf,
            req,
            res,
            response.hook!,
            response.streaming!,
          )
          await call.onAccepted()
          call.on(
            IncomingCallEvent.StateChanged,
            ({ state }: { state: IncomingCallState }) => {
              this.emit(SipCallEvent.StateChanged, [call_id, state])
              switch (state) {
                case IncomingCallState.Canceled:
                case IncomingCallState.Ended:
                  this.incoming_calls.delete(call_id)
                  break
              }
            },
          )
          this.incoming_calls.set(call_id, call)
          break
        }
        case 'Rejected': {
          res.send(486) //busy now
          break
        }
        case 'Ringing': {
          res.send(180) //ringing
          const call = new IncomingCall(
            this.srf,
            req,
            res,
            response.hook!,
            response.streaming!,
          )
          call.on(
            IncomingCallEvent.StateChanged,
            ({ state }: { state: IncomingCallState }) => {
              this.emit(SipCallEvent.StateChanged, [call_id, state])
              switch (state) {
                case IncomingCallState.Canceled:
                case IncomingCallState.Rejected:
                case IncomingCallState.Ended:
                  this.incoming_calls.delete(call_id)
                  break
              }
            },
          )
          this.incoming_calls.set(call_id, call)
          break
        }
        default: {
          res.send(486) //busy now
          break
        }
      }
    } catch (err) {
      console.error('Feedback status error', call_id, err)
      res.send(480) //hook error
    }
  }
}
