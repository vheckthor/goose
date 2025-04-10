import { enable_extension } from './enable_extension';

// Export all tools
export const tools = {
  enable_extension: {
    name: 'enable_extension',
    description: 'Enable a Goose extension',
    function: enable_extension,
    schema: {
      type: 'object',
      oneOf: [
        {
          properties: {
            type: { const: 'sse' },
            name: { type: 'string', description: 'Name of the extension' },
            description: { type: 'string', description: 'Description of the extension' },
            uri: { type: 'string', description: 'URI for SSE extension' },
            env_keys: {
              type: 'array',
              items: { type: 'string' },
              description: 'Environment variable keys',
            },
            timeout: { type: 'number', description: 'Timeout in seconds' },
          },
          required: ['type', 'name', 'uri'],
          additionalProperties: false,
        },
        {
          properties: {
            type: { const: 'stdio' },
            name: { type: 'string', description: 'Name of the extension' },
            description: { type: 'string', description: 'Description of the extension' },
            cmd: { type: 'string', description: 'Command for stdio extension' },
            args: { type: 'array', items: { type: 'string' }, description: 'Command arguments' },
            env_keys: {
              type: 'array',
              items: { type: 'string' },
              description: 'Environment variable keys',
            },
            timeout: { type: 'number', description: 'Timeout in seconds' },
          },
          required: ['type', 'name', 'cmd'],
          additionalProperties: false,
        },
        {
          properties: {
            type: { const: 'builtin' },
            name: { type: 'string', description: 'Name of the extension' },
            description: { type: 'string', description: 'Description of the extension' },
            env_keys: {
              type: 'array',
              items: { type: 'string' },
              description: 'Environment variable keys',
            },
            timeout: { type: 'number', description: 'Timeout in seconds' },
          },
          required: ['type', 'name'],
          additionalProperties: false,
        },
      ],
    },
  },
};
