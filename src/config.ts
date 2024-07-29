import path from 'path'
import * as dotenv from 'dotenv'

const envPath = path.join(process.cwd(), '.env')
dotenv.config({
  path: envPath,
  override: true,
})

export const ENV = process.env.ENV || 'develop'
export const RUN_MODE = process.env.RUN_MODE || 'rtp'

export const DRACHTIO_CONFIG = {
  host: process.env.DRACHTIO_HOST || '127.0.0.1',
  port: parseInt(process.env.DRACHTIO_PORT || '9022'),
  secret: process.env.DRACHTIO_SECRET || '',
}

export const RTP_ENGINE_CONFIG = {
  host: process.env.RTP_ENGINE_HOST || '127.0.0.1',
  port: parseInt(process.env.RTP_ENGINE_PORT || '22222'),
}

export const ATM0S_CONFIG = {
  GATEWAY: process.env.ATM0S_GATEWAY || 'http://127.0.0.1:3002',
  SECRET: process.env.ATM0S_SECRET || 'insecure',
}
