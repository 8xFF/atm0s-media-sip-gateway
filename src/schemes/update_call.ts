import { FastifySchema } from 'fastify'
import { CallAction } from 'sip/call/lib'

export interface UpdateCallRequest {
  state: CallAction
}

export const UPDATE_CALL_SCHEMA: FastifySchema = {
  description: 'Make outgoing call',
  summary: 'Make outgoing call',
  params: {
    call_id: {
      type: 'string',
    },
  },
  headers: {
    'X-API-Key': {
      type: 'string',
      description: 'API key for authentication',
    },
  },
  body: {
    type: 'object',
    properties: {
      state: {
        type: 'string',
        enum: ['Cancel', 'Reject', 'Accept', 'End'],
      },
    },
    required: ['state'],
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
      },
      required: ['status'],
      additionalProperties: false,
    },
  },
}
