import React, {
    createContext,
    useContext,
    useState,
    ReactNode,
    useEffect,
    useCallback,
} from 'react';

import { toast } from 'react-toastify';
// Import your existing helper functions
import {
    addExtension as addExtensionToServer,
    removeExtension as removeExtensionFromServer,
    // etc...
} from '../../extensions';

import { BUILT_IN_EXTENSIONS } from '../../extensions'; // adjust paths
import DEFAULT_SETTINGS from './Settings'
import type { FullExtensionConfig } from '../../extensions';
import { Settings as SettingsType } from './types';

// Shape of the context
interface SettingsContextValue {
    settings: SettingsType;  // entire user settings, which includes `extensions`
    setSettings: React.Dispatch<React.SetStateAction<SettingsType>>;

    toggleExtension: (extensionId: string) => Promise<void>;
    removeExtensionCompletely: (extensionId: string) => Promise<void>;
    addExtensionManually: (extension: FullExtensionConfig) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | undefined>(undefined);

export const SettingsProvider = ({ children }: { children: ReactNode }) => {
    const [settings, setSettings] = useState<SettingsType>(() => {
        // On first load, try to get from local storage
        const saved = localStorage.getItem('user_settings');
        if (!saved) {
            // If none, fallback to your default
            console.log("settings", DEFAULT_SETTINGS)
            return DEFAULT_SETTINGS;
        }
        try {
            console.log("settings", JSON.parse(saved))
            return JSON.parse(saved);
        } catch (err) {
            console.error('Failed to parse user_settings from localStorage, using DEFAULT_SETTINGS', err);
            return DEFAULT_SETTINGS;
        }
    });

    /**
     * Helper to persist to localStorage
     */
    const persistSettings = useCallback((newSettings: SettingsType) => {
        localStorage.setItem('user_settings', JSON.stringify(newSettings));
        setSettings(newSettings);
        // Fire off the Electron event if that’s desired:
    }, []);

    // Provide a method for toggling an extension’s enabled state
    const toggleExtension = useCallback(
        async (extensionId: string) => {
            const extension = settings.extensions.find((ext) => ext.id === extensionId);
            if (!extension) return;

            const newEnabled = !extension.enabled;
            const originalSettings = structuredClone(settings);

            // Optimistically update local state
            const updatedSettings: SettingsType = {
                ...settings,
                extensions: settings.extensions.map((ext) =>
                    ext.id === extensionId ? { ...ext, enabled: newEnabled } : ext
                ),
            };

            persistSettings(updatedSettings);

            // Now notify the server side
            let response: Response;
            try {
                if (newEnabled) {
                    response = await addExtensionToServer(extension);
                } else {
                    // NOTE: do not *remove* from local storage if it’s built-in, only disable:
                    response = await removeExtensionFromServer(extension.name);
                }
                if (!response.ok) {
                    // revert local changes if server call fails
                    persistSettings(originalSettings);
                }
            } catch (err) {
                console.error('Error toggling extension:', err);
                persistSettings(originalSettings);
            }
        },
        [settings, persistSettings]
    );

    // Provide a method for completely removing an extension from local storage
    const removeExtensionCompletely = useCallback(
        async (extensionId: string) => {
            const extension = settings.extensions.find((ext) => ext.id === extensionId);
            if (!extension) return;

            // If it’s built-in, do not remove it—only “disable”:
            const isBuiltIn = BUILT_IN_EXTENSIONS.some((b) => b.id === extension.id);
            if (isBuiltIn) {
                toast.error(`Cannot remove built-in extension: ${extension.name}`);
                return;
            }

            const originalSettings = structuredClone(settings);

            // Filter out from local storage
            const updatedSettings: SettingsType = {
                ...settings,
                extensions: settings.extensions.filter((ext) => ext.id !== extensionId),
            };
            persistSettings(updatedSettings);

            // Also remove from the server
            try {
                const response = await removeExtensionFromServer(extension.name, true);
                if (!response.ok) {
                    persistSettings(originalSettings); // revert if server call fails
                } else {
                    toast.success(`Removed extension: ${extension.name}`);
                }
            } catch (err) {
                console.error('Error removing extension:', err);
                persistSettings(originalSettings);
            }
        },
        [settings, persistSettings]
    );

    // Provide a method for manually adding a new extension
    const addExtensionManually = useCallback(
        async (extension: FullExtensionConfig) => {
            const originalSettings = structuredClone(settings);

            // Add it to local storage
            const updatedSettings: SettingsType = {
                ...settings,
                extensions: [...settings.extensions, extension],
            };
            persistSettings(updatedSettings);

            // Also add to server
            try {
                const response = await addExtensionToServer(extension);
                if (!response.ok) {
                    // revert local changes if server call fails
                    persistSettings(originalSettings);
                }
            } catch (err) {
                console.error('Error adding extension:', err);
                persistSettings(originalSettings);
            }
        },
        [settings, persistSettings]
    );

    // Listen for external “settings-updated” events from Electron and reload from localStorage
    // (keeps your context in sync if something changed behind the scenes)
    useEffect(() => {
        const handleSettingsUpdate = () => {
            const saved = localStorage.getItem('user_settings');
            if (saved) {
                try {
                    persistSettings(JSON.parse(saved));
                } catch (err) {
                    console.error('Error re-parsing user_settings in handleSettingsUpdate:', err);
                }
            }
        };

        window.electron.on('settings-updated', handleSettingsUpdate);
        return () => {
            window.electron.off('settings-updated', handleSettingsUpdate);
        };
    }, [persistSettings]);

    return (
        <SettingsContext.Provider
            value={{
                settings,
                setSettings, // in case you need direct modifications
                toggleExtension,
                removeExtensionCompletely,
                addExtensionManually,
            }}
        >
            {children}
        </SettingsContext.Provider>
    );
};

// Hook for consuming the context
export const useSettings = (): SettingsContextValue => {
    const context = useContext(SettingsContext);
    if (!context) {
        throw new Error('useSettings must be used within a SettingsProvider');
    }
    return context;
};
