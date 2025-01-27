import { useState, useEffect } from 'react';
import {
  FullExtensionConfig,
  addExtension,
  removeExtension,
  BUILT_IN_EXTENSIONS,
} from '../../../extensions';
import { toast } from 'react-toastify';

export function extractCommand(link: string): string {
  const url = new URL(link);
  const cmd = url.searchParams.get('cmd') || 'Unknown Command';
  const args = url.searchParams.getAll('arg').map(decodeURIComponent);

  // Combine the command and its arguments into a reviewable format
  return `${cmd} ${args.join(' ')}`.trim();
}

export function extractExtensionName(link: string): string {
  const url = new URL(link);
  const name = url.searchParams.get('name');
  return name ? decodeURIComponent(name) : 'Unknown Extension';
}

export function useExtensions() {
  const [extensions, setExtensions] = useState<FullExtensionConfig[]>(() => {
    const saved = localStorage.getItem('user_settings');
    const currentSettings = saved ? JSON.parse(saved).extensions : BUILT_IN_EXTENSIONS;

    // Ensure all built-in extensions are present
    BUILT_IN_EXTENSIONS.forEach((builtIn) => {
      if (!currentSettings.some((ext: FullExtensionConfig) => ext.id === builtIn.id)) {
        currentSettings.push(builtIn);
      }
    });

    return currentSettings;
  });

  useEffect(() => {
    localStorage.setItem('user_settings', JSON.stringify({ extensions }));
  }, [extensions]);

  const toggleExtension = async (extensionId: string) => {
    const extension = extensions.find((ext) => ext.id === extensionId);
    if (!extension) return;

    const updatedExtension = { ...extension, enabled: !extension.enabled };

    setExtensions((prev) =>
        prev.map((ext) => (ext.id === extensionId ? updatedExtension : ext))
    );

    const response = extension.enabled
        ? await removeExtension(extension.name)
        : await addExtension(extension);

    if (!response.ok) {
      toast.error(`Failed to update ${extension.name}`);
      setExtensions((prev) =>
          prev.map((ext) => (ext.id === extensionId ? extension : ext))
      );
    }
  };

  const removeExtensionById = async (extensionId: string) => {
    const extension = extensions.find((ext) => ext.id === extensionId);
    if (!extension) return;

    const response = await removeExtension(extension.name);
    if (response.ok) {
      setExtensions((prev) => prev.filter((ext) => ext.id !== extensionId));
      toast.success(`${extension.name} removed successfully`);
    } else {
      toast.error(`Failed to remove ${extension.name}`);
    }
  };

  return { extensions, toggleExtension, removeExtensionById, setExtensions };
}

export async function getStoredExtensionsAndBuiltIns(): Promise<FullExtensionConfig[]> {
  try {
    const userSettingsStr = localStorage.getItem('user_settings');
    const userSettings = userSettingsStr
        ? JSON.parse(userSettingsStr)
        : { extensions: [] };

    const { extensions = [] } = userSettings;

    console.log('[loadStoredExtensionsAndBuiltIns]: found these extensions in localStorage:', userSettings)
    console.log('[loadStoredExtensionsAndBuiltIns]: this is the value of extensions: ', extensions)

    // handle builtins -- add them to list of all extensions for saving downstream in localstorage
    const allExtensions = await ensureBuiltInsAreStoredAndAdded(extensions)

    return allExtensions; // Return the full list of extensions
  } catch (error) {
    console.error('Error loading and activating extensions from localStorage: ', error);
    return [];
  }
}

async function ensureBuiltInsAreStoredAndAdded(extensions) {
  let allExtensions: FullExtensionConfig[] = [...extensions];
  console.log("[ensureBuiltInsAreStoredAndAdded] going through builtins")

  // Ensure built-in extensions are stored if missing
  for (const builtIn of BUILT_IN_EXTENSIONS) {
    console.log(builtIn)
    const exists = extensions.some((ext: FullExtensionConfig) => ext.id === builtIn.id);
    if (!exists) {
      console.log("Adding builtin", builtIn.name)
      allExtensions.push(builtIn); // Add to the return list
    }
  }

  console.log("full extensions list:", allExtensions)
  return allExtensions
}
