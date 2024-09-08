import EventEmitter from 'events'

// This class manage websocket, when all ws users in a call disconnected, the call will automatically destroy
export class WsGateway extends EventEmitter {
  calls: Map<string, Call> = new Map()
  constructor() {
    super()
  }

  onConnected = (call_id: string, conn_id: string, ws: WebSocket) => {
    let call = this.calls.get(call_id)
    if (!call) {
      console.log('create call', call_id)
      call = new Call(call_id)
      this.calls.set(call_id, call)
      call.on(CallEvent.Started, () => {
        this.emit(CallEvent.Started, { call_id })
      })
      call.on(CallEvent.Stopped, () => {
        this.emit(CallEvent.Stopped, { call_id })
        this.calls.delete(call_id)
      })
    }

    call!.addConnection(conn_id, ws)
  }

  fire = (call_id: string, data: object) => {
    const call = this.calls.get(call_id)
    if (call) {
      call.fire(data)
    } else {
      console.warn('Call not found', call_id)
    }
  }
}

export enum CallEvent {
  Started = 'Started',
  Stopped = 'Stopped',
}

class Call extends EventEmitter {
  users: Map<string, WebSocket> = new Map()

  constructor(private call_id: string) {
    super()
  }

  addConnection(conn_id: string, ws: WebSocket) {
    console.log('call', this.call_id, 'added conn', conn_id)
    this.users.set(conn_id, ws)
    if (this.users.size == 1) {
      this.emit(CallEvent.Started)
    }
    ws.onclose = () => {
      console.log('call', this.call_id, 'removed conn', conn_id)
      this.users.delete(conn_id)
      if (this.users.size == 0) {
        this.emit(CallEvent.Stopped)
      }
    }
  }

  fire(data: object) {
    this.users.forEach((socket) => {
      socket.send(JSON.stringify(data))
    })
  }
}
