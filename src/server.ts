import {
  ALLOWED_NUMBERS_SYNC,
  DRACHTIO_CONFIG,
  ENABLE_REGISTER,
  INCOMING_CALL_HOOK,
  PORT,
  SECRET,
} from 'config'
import { SipCallEvent, SipGateway } from 'sip/lib'
import Fastify from 'fastify'
import { fastifySwaggerUi } from '@fastify/swagger-ui'
import { fastifySwagger } from '@fastify/swagger'
import { fastifyWebsocket } from '@fastify/websocket'
import { MAKE_CALL_SCHEMA, MakeCallRequest } from './schemes/make_call'
import { UPDATE_CALL_SCHEMA, UpdateCallRequest } from './schemes/update_call'
import { CallEvent, WsGateway } from 'ws_gateway'
import { errToString } from 'utils'

const fastify = Fastify({
  logger: true,
})

async function boot() {
  const wsGw = new WsGateway()
  const sipGw = new SipGateway(
    DRACHTIO_CONFIG,
    INCOMING_CALL_HOOK,
    ENABLE_REGISTER,
    ALLOWED_NUMBERS_SYNC,
  )
  await sipGw.connect()

  wsGw.on(CallEvent.Started, ({ call_id }: { call_id: string }) => {
    console.log('Call', call_id, 'has websocket connected')
  })

  wsGw.on(CallEvent.Stopped, ({ call_id }: { call_id: string }) => {
    console.log('Call', call_id, 'closed all websocket connections')
    sipGw.callAction(call_id, 'ForceEnd')
  })

  sipGw.on(SipCallEvent.StateChanged, ([call_id, status]) => {
    console.log('Call', call_id, 'updated to new state', status)
    wsGw.fire(call_id, status)
  })

  await fastify.register(fastifyWebsocket)
  await fastify.register(fastifySwagger)
  await fastify.register(fastifySwaggerUi, {
    routePrefix: '/docs',
    uiConfig: {
      docExpansion: 'full',
      deepLinking: false,
    },
    uiHooks: {
      onRequest: function (request, reply, next) {
        next()
      },
      preHandler: function (request, reply, next) {
        next()
      },
    },
    staticCSP: true,
    transformStaticCSP: (header) => header,
    transformSpecification: (swaggerObject) => {
      return swaggerObject
    },
    transformSpecificationClone: true,
  })

  fastify.post('/call', { schema: MAKE_CALL_SCHEMA }, async (req) => {
    if (req.headers['x-api-key'] != SECRET) {
      console.log(req.headers, SECRET)
      return { status: false, error: 'AUTHENTICATION_ERROR' }
    }

    try {
      const body = req.body as MakeCallRequest
      const call_id = await sipGw.makeCall(
        body.sip_server,
        body.sip_auth,
        body.from_number,
        body.to_number,
        body.hook,
        body.streaming,
      )
      console.log('Male Call success', call_id)
      const token = 'fake-token'
      return {
        status: true,
        data: { call_id, ws: '/ws/call/' + call_id + '?token=' + token },
      }
    } catch (e: any) {
      console.log('Make Call error', e)
      return {
        status: false,
        error: 'MAKE_CALL_ERROR',
        message: errToString(e),
      }
    }
  })

  fastify.put('/call/:call_id', { schema: UPDATE_CALL_SCHEMA }, async (req) => {
    const { call_id } = req.params as any
    const body = req.body as UpdateCallRequest
    try {
      const res = await sipGw.callAction(call_id, body.action)
      console.log('Update Call success', call_id)
      return res
    } catch (e: any) {
      console.log('Update Call error', e)
      return {
        status: false,
        error: 'UPDATE_CALL_ERROR',
        message: errToString(e),
      }
    }
  })

  fastify.get('/ws/call/:call_id', { websocket: true }, (socket, req) => {
    const { call_id } = req.params as any
    const { token } = req.query as { token: string }
    // TODO validate token
    wsGw.onConnected(call_id, req.id, socket)
  })

  // Run the server!
  try {
    console.log('starting fastify with port', PORT)
    await fastify.listen({ host: '0.0.0.0', port: PORT })
  } catch (err) {
    fastify.log.error(err)
    process.exit(1)
  }
}

boot()
