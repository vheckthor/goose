import { getApiUrl, getSecretKey } from '../config';
import {addExtension, FullExtensionConfig, loadAndAddStoredExtensions} from '../extensions';
import { GOOSE_PROVIDER } from '../env_vars';
import { Model } from '../components/settings/models/ModelContext';
import {useStoredExtensions} from "../components/settings/extensions/StoredExtensionsContext";

export function getStoredProvider(config: any): string | null {
  console.log('config goose provider', config.GOOSE_PROVIDER);
  console.log('local storage goose provider', localStorage.getItem(GOOSE_PROVIDER));
  return config.GOOSE_PROVIDER || localStorage.getItem(GOOSE_PROVIDER);
}

export function getStoredModel(): string | null {
  const storedModel = localStorage.getItem('GOOSE_MODEL'); // Adjust key name if necessary
  console.log('local storage goose model', storedModel);

  if (storedModel) {
    try {
      const modelInfo: Model = JSON.parse(storedModel);
      return modelInfo.name || null; // Return name if it exists, otherwise null
    } catch (error) {
      console.error('Error parsing GOOSE_MODEL from local storage:', error);
      return null; // Return null if parsing fails
    }
  }

  return null; // Return null if storedModel is not found
}

export interface Provider {
  id: string; // Lowercase key (e.g., "openai")
  name: string; // Provider name (e.g., "OpenAI")
  description: string; // Description of the provider
  models: string[]; // List of supported models
  requiredKeys: string[]; // List of required keys
}

export async function getProvidersList(): Promise<Provider[]> {
  const response = await fetch(getApiUrl('/agent/providers'), {
    method: 'GET',
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch providers: ${response.statusText}`);
  }

  const data = await response.json();
  console.log('Raw API Response:', data); // Log the raw response

  // Format the response into an array of providers
  return data.map((item: any) => ({
    id: item.id, // Root-level ID
    name: item.details?.name || 'Unknown Provider', // Nested name in details
    description: item.details?.description || 'No description available.', // Nested description
    models: item.details?.models || [], // Nested models array
    requiredKeys: item.details?.required_keys || [], // Nested required keys array
  }));
}

const addAgent = async (provider: string, model: string) => {
  const response = await fetch(getApiUrl('/agent'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({ provider: provider, model: model }),
  });

  if (!response.ok) {
    throw new Error(`Failed to add agent: ${response.statusText}`);
  }

  return response;
};

export const useInitializeSystem = () => {
  const { storeExtensionConfig } = useStoredExtensions();
  console.log("in use initialize system")

  const initializeSystem = async (provider: string, model: string) => {
    try {
      console.log('Initializing agent with provider:', provider, 'model:', model);
      await addAgent(provider.toLowerCase(), model);
      // try to add the stored extensions that are enabled
      const storedExtensions = useStoredExtensions()
    } catch (error) {
      console.error('Failed to initialize agent:', error);
      throw error;
    }
  };

  return initializeSystem;
};

export const initializeSystem = async (provider: string, model: string, storedExtensions: FullExtensionConfig[]) => {
  try {
    console.log('Initializing agent with provider:', provider, 'model:', model);
    await addAgent(provider.toLowerCase(), model);
    // try to install in the backend the stored extensions that are enabled
    for (const extension of storedExtensions) {
      if (extension.enabled) {
        console.log("[InitializeSytem]: attempting to add extension:", extension.name)
        addExtension(extension)
      }
    }

  } catch (error) {
    console.error('Failed to initialize agent:', error);
    throw error;
  }
};
