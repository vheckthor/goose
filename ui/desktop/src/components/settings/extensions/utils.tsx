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
