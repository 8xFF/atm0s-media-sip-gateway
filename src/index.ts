import { DRACHTIO_CONFIG } from 'config'
import Srf from 'drachtio-srf'
import { handleInvite } from 'handlers/invite'
import { handleRegister } from 'handlers/register'
import { SipDatabase } from 'storage'

export const STORAGE = new SipDatabase()

export const srf = new Srf()
srf.connect(DRACHTIO_CONFIG)
srf
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
const srf2 = srf as any
srf2.register(handleRegister)
srf.invite(handleInvite)
