import { addExtension } from '../extensions';
import { DEFAULT_EXTENSION_TIMEOUT } from '../extensions';
import { FullExtensionConfig } from '../extensions';

interface BaseExtensionArgs {
  name: string;
  description?: string;
  env_keys?: string[];
  timeout?: number;
}

interface SseExtensionArgs extends BaseExtensionArgs {
  type: 'sse';
  uri: string;
}

interface StdioExtensionArgs extends BaseExtensionArgs {
  type: 'stdio';
  cmd: string;
  args?: string[];
}

interface BuiltinExtensionArgs extends BaseExtensionArgs {
  type: 'builtin';
}

type EnableExtensionArgs = SseExtensionArgs | StdioExtensionArgs | BuiltinExtensionArgs;

export async function enable_extension(args: EnableExtensionArgs): Promise<string> {
  try {
    // Create base config
    const baseConfig = {
      id: `ext_${Date.now()}`,
      name: args.name,
      description: args.description || args.name,
      enabled: true,
      timeout: args.timeout || DEFAULT_EXTENSION_TIMEOUT,
      env_keys: args.env_keys || [],
    };

    // Create type-specific config
    let config: FullExtensionConfig;

    switch (args.type) {
      case 'sse': {
        const sseArgs = args as SseExtensionArgs;
        config = {
          ...baseConfig,
          type: 'sse',
          uri: sseArgs.uri,
        };
        break;
      }
      case 'stdio': {
        const stdioArgs = args as StdioExtensionArgs;
        config = {
          ...baseConfig,
          type: 'stdio',
          cmd: stdioArgs.cmd,
          args: stdioArgs.args || [],
        };
        break;
      }
      case 'builtin': {
        config = {
          ...baseConfig,
          type: 'builtin',
        };
        break;
      }
      default: {
        // This should never happen since we've handled all possible types
        throw new Error(`Unsupported extension type: ${(args as EnableExtensionArgs).type}`);
      }
    }

    // Add the extension
    const response = await addExtension(config);

    if (!response.ok) {
      const data = await response.json();
      throw new Error(data.message || 'Failed to enable extension');
    }

    return `Successfully enabled ${args.name} extension`;
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    throw new Error(`Failed to enable extension: ${message}`);
  }
}
