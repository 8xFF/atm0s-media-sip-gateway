import path from 'path'
import * as dotenv from 'dotenv'

const envPath = path.join(process.cwd(), '.env')
dotenv.config({
  path: envPath,
  override: true,
})

export const ENV = process.env.ENV || 'develop'

export const DRACHTIO_CONFIG = {
  host: process.env.DRACHTIO_HOST || '127.0.0.1',
  port: parseInt(process.env.DRACHTIO_PORT || '9022'),
  secret: process.env.DRACHTIO_SECRET || '',
  sip_server: process.env.DRACHTIO_SIP_SERVER || '127.0.0.1:5060',
}

export const INCOMING_CALL_HOOK =
  process.env.INCOMING_CALL_HOOK || 'http://localhost:3000'

export const ALLOWED_NUMBERS_SYNC = process.env.ALLOWED_NUMBERS_SYNC

export const ENABLE_REGISTER = process.env.ENABLE_REGISTER === 'true'

export const SECRET = process.env.SECRET || 'insecure'
export const PORT = parseInt(process.env.PORT || '5000')

export const ATM0S_CONFIG = {
  gateway: process.env.ATM0S_GATEWAY || 'http://127.0.0.1:3000',
  secret: process.env.ATM0S_SECRET || 'insecure',
}
