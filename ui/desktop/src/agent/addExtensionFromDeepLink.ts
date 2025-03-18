import { toast } from 'react-toastify';
import { ExtensionConfig } from '../api';
import { FullExtensionConfig } from '../extensions';
import { View } from '../App';
import { SettingsViewOptions } from '../components/settings_v2/SettingsView';

const DEFAULT_EXTENSION_TIMEOUT = 300;

function handleError(message: string, shouldThrow = false): void {
  toast.error(message, { autoClose: false });
  console.error(message);
  if (shouldThrow) {
    throw new Error(message);
  }
}

const parse = (url) => {
  if (!url.startsWith('goose://extension')) {
    handleError(
      'Failed to install extension: Invalid URL: URL must use the goose://extension scheme'
    );
    return;
  }

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== 'goose:') {
    handleError(
      'Failed to install extension: Invalid protocol: URL must use the goose:// scheme',
      true
    );
  }

  // Check that all required fields are present and not empty
  const requiredFields = ['name', 'description'];

  for (const field of requiredFields) {
    const value = parsedUrl.searchParams.get(field);
    if (!value || value.trim() === '') {
      handleError(
        `Failed to install extension: The link is missing required field '${field}'`,
        true
      );
    }
  }

  const cmd = parsedUrl.searchParams.get('cmd');
  if (!cmd) {
    handleError("Failed to install extension: Missing required 'cmd' parameter in the URL", true);
  }

  // Validate that the command is one of the allowed commands
  const allowedCommands = ['npx', 'uvx', 'goosed'];
  if (!allowedCommands.includes(cmd)) {
    handleError(
      `Failed to install extension: Invalid command: ${cmd}. Only ${allowedCommands.join(', ')} are allowed.`,
      true
    );
  }

  // Check for security risk with npx -c command
  const args = parsedUrl.searchParams.getAll('arg');
  if (cmd === 'npx' && args.includes('-c')) {
    handleError(
      'Failed to install extension: npx with -c argument can lead to code injection',
      true
    );
  }

  return { parsedUrl, cmd, args };
};

export const addExtensionFromDeepLink = async (
  url: string,
  setView: (view: View, options: SettingsViewOptions) => void,
  addExtensionToAgent: (extension: ExtensionConfig, silent?: boolean) => Promise<Response>
) => {
  const { parsedUrl, cmd, args } = parse(url);

  const envList = parsedUrl.searchParams.getAll('env');
  const id = parsedUrl.searchParams.get('id');
  const name = parsedUrl.searchParams.get('name');
  const description = parsedUrl.searchParams.get('description');
  const timeout = parsedUrl.searchParams.get('timeout');

  // split env based on delimiter to a map
  const envs = envList.reduce(
    (acc, env) => {
      const [key, value] = env.split('=');
      acc[key] = value;
      return acc;
    },
    {} as Record<string, string>
  );

  // Create a ExtensionConfig from the URL parameters
  // Parse timeout if provided, otherwise use default
  const parsedTimeout = timeout ? parseInt(timeout, 10) : null;

  const config: FullExtensionConfig = {
    id,
    name,
    type: 'stdio',
    cmd,
    args,
    description,
    enabled: true,
    env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : [],
    timeout:
      parsedTimeout !== null && !isNaN(parsedTimeout) && Number.isInteger(parsedTimeout)
        ? parsedTimeout
        : DEFAULT_EXTENSION_TIMEOUT,
  };

  const envVarsRequired = (config: FullExtensionConfig) => config.env_keys?.length > 0;

  const { needsAdditionalArgs } = addExtensionToAgent(config);

  // Check if extension requires env vars and go to settings if so
  if (envVarsRequired(config)) {
    console.log('Environment variables required, redirecting to settings');
    setView('settings', { extensionId: config.id, showEnvVars: true });
  }

  return;
  // Store the extension config regardless of env vars status
  storeExtensionConfig(config);

  // If no env vars are required, proceed with extending Goosed
  await addExtension(config);
};
