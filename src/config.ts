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
}

export const INCOMING_CALL_HOOK =
  process.env.INCOMING_CALL_HOOK || 'http://localhost:4222/hook/sip/call'

export const ALLOWED_NUMBERS_SYNC =
  process.env.ALLOWED_NUMBERS_SYNC || 'http://localhost:4222/hook/sip/numbers'

export const SECRET = process.env.SECRET || 'insecure'
export const PORT = parseInt(process.env.PORT || '5000')
