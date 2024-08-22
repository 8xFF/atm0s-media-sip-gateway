import { FastifySchema } from 'fastify'

export interface MakeCallRequest {
  sip_server: string
  from_number: string
  to_number: string
  hook: string
  streaming: {
    room_id: string
    peer_id: string
  }
}

export interface MakeCallResponse {
  call_id: string
}

export const MAKE_CALL_SCHEMA: FastifySchema = {
  description: 'Make outgoing call',
  summary: 'Make outgoing call',
  params: {},
  headers: {
    'X-API-Key': {
      type: 'string',
      description: 'API key for authentication',
    },
  },
  body: {
    type: 'object',
    properties: {
      sip_server: {
        type: 'string',
      },
      from_number: {
        type: 'string',
      },
      to_number: {
        type: 'string',
      },
      hook: {
        type: 'string',
      },
      streaming: {
        type: 'object',
        properties: {
          room_id: {
            type: 'string',
          },
          peer_id: {
            type: 'string',
          },
        },
        required: ['room_id', 'peer_id'],
      },
    },
    required: ['from_number', 'to_number', 'hook', 'streaming'],
  },
  response: {
    200: {
      type: 'object',
      properties: {
        status: {
          type: 'boolean',
        },
        error: {
          type: ['string', 'null'],
        },
        message: {
          type: ['string', 'null'],
        },
        data: {
          type: 'object',
          properties: {
            call_id: {
              type: 'string',
            },
          },
          required: ['call_id'],
        },
      },
      required: ['status'],
      additionalProperties: false,
    },
  },
}
