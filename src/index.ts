import {
  ALLOWED_NUMBERS_SYNC,
  ATM0S_CONFIG,
  DRACHTIO_CONFIG,
  ENABLE_REGISTER,
  INCOMING_CALL_HOOK,
  SECRET,
} from 'config'
import { SipGateway } from 'sip/lib'
import Fastify from 'fastify'
import { fastifySwaggerUi } from '@fastify/swagger-ui'
import { fastifySwagger } from '@fastify/swagger'
import { MAKE_CALL_SCHEMA, MakeCallRequest } from 'schemes/make_call'
import { UPDATE_CALL_SCHEMA, UpdateCallRequest } from 'schemes/update_call'

const fastify = Fastify({
  logger: true,
})

async function boot() {
  const sip = new SipGateway(
    DRACHTIO_CONFIG,
    INCOMING_CALL_HOOK,
    ATM0S_CONFIG,
    ENABLE_REGISTER,
    ALLOWED_NUMBERS_SYNC,
  )
  await sip.connect()

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
    if (req.headers['X-API-Key'] != SECRET) {
      return { status: false, error: 'AUTHENTICATION_ERROR' }
    }

    try {
      const body = req.body as MakeCallRequest
      const call_id = await sip.makeCall(
        body.sip_server,
        body.from_number,
        body.to_number,
        body.hook,
        body.streaming.room_id,
        body.streaming.peer_id,
      )
      console.log('Male Call success', call_id)
      return { status: true, data: { call_id } }
    } catch (e: any) {
      console.log('Make Call error', e)
      return { status: false, error: 'MAKE_CALL_ERROR', message: e.to_string() }
    }
  })

  fastify.put('/call/:call_id', { schema: UPDATE_CALL_SCHEMA }, async (req) => {
    const { call_id } = req.params as any
    const body = req.body as UpdateCallRequest
    try {
      await sip.callAction(call_id, body.state)
      console.log('Update Call success', call_id)
      return { status: true }
    } catch (e: any) {
      console.log('Update Call error', e)
      return {
        status: false,
        error: 'UPDATE_CALL_ERROR',
        message: e.to_string(),
      }
    }
  })

  // Run the server!
  try {
    await fastify.listen({ host: '0.0.0.0', port: 5000 })
  } catch (err) {
    fastify.log.error(err)
    process.exit(1)
  }
}

boot()
