export type CallAction = 'Cancel' | 'Reject' | 'Accept' | 'End'

export interface CallCfg {
  hook: string
  room: string
  peer: string
  call_id: string
  from_number: string
  to_number: string
}

export interface Call {
  doAction(action: CallAction): Promise<void>
}
