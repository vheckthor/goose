import { addExtension, DEFAULT_EXTENSION_TIMEOUT, type FullExtensionConfig } from '../extensions';

// The tool implementation
export async function enable_extension(args: Record<string, unknown>): Promise<string> {
  try {
    const extension_name = args.extension_name as string;
    if (!extension_name) {
      throw new Error('extension_name is required');
    }

    // Get config by name and enable it
    const config: FullExtensionConfig = {
      id: `ext_${Date.now()}`,
      name: extension_name,
      description: extension_name,
      enabled: true,
      timeout: DEFAULT_EXTENSION_TIMEOUT,
      env_keys: [],
      type: 'builtin'
    };

    const response = await addExtension(config);
    if (!response.ok) {
      const data = await response.json();
      throw new Error(data.message || 'Failed to enable extension');
    }

    return `Successfully enabled ${extension_name} extension`;
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    throw new Error(`Failed to enable extension: ${message}`);
  }
}

// Export all tools
export const tools = {
  enable_extension: {
    name: 'enable_extension',
    description: 'Enable extension',
    function: enable_extension,
    schema: {
      type: 'object',
      required: ['extension_name'],
      properties: {
        extension_name: {
          type: 'string',
          description: 'Name of the extension to install',
        }
      },
    },
  },
};

// Frontend extension configuration type
interface FrontendConfig {
  name: string;
  type: 'frontend';
  tools: Array<{
    name: string;
    description: string;
    schema: {
      type: string;
      required: string[];
      properties: Record<string, unknown>;
    };
  }>;
  instructions: string;
}

// Frontend configuration
export const FRONTEND_CONFIG: FrontendConfig = {
  name: 'frontendtools',
  type: 'frontend',
  tools: [tools.enable_extension],
  instructions: 'Frontend tools that are invoked by the frontend',
};
