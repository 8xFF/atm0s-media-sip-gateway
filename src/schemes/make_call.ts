import { FastifySchema } from 'fastify'

export interface StreamingInfo {
  gateway: string
  token: string
}

export interface SipAuth {
  username: string
  password: string
}

export interface MakeCallRequest {
  sip_server: string
  sip_auth?: SipAuth
  from_number: string
  to_number: string
  hook: string
  streaming: StreamingInfo
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
      sip_auth: {
        type: 'object',
        properties: {
          username: {
            type: 'string',
          },
          password: {
            type: 'string',
          },
        },
        required: ['username', 'password'],
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
          gateway: {
            type: 'string',
          },
          token: {
            type: 'string',
          },
          room: {
            type: 'string',
          },
          peer: {
            type: 'string',
          },
        },
        required: ['room', 'peer', 'gateway', 'token'],
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
            ws: {
              type: 'string',
            },
          },
          required: ['call_id', 'ws'],
        },
      },
      required: ['status'],
      additionalProperties: false,
    },
  },
}
