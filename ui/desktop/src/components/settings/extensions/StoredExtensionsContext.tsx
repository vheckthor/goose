import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import {FullExtensionConfig, BUILT_IN_EXTENSIONS, loadAndAddStoredExtensions} from '../../../extensions';
import {addExtension as addExtensionToBackend} from "../../../extensions";
import {removeExtension as removeExtensionFromBackend} from "../../../extensions";

const StoredExtensionsContext = createContext<
    | {
    installedExtensions: FullExtensionConfig[];
    toggleExtension: (extension: string) => void;
    removeExtension: (extension: string) => void;
    addExtension: (extension: FullExtensionConfig) => Promise<void>;
    storeExtensionConfig: (extension: FullExtensionConfig) => void;
}
    | undefined
>(undefined);

export const StoredExtensionsProvider = ({ children }: { children: ReactNode }) => {
    const [storedExtensions, setStoredExtensions] = useState<FullExtensionConfig[]>([]);
    console.log("herehehe")
    // Load extensions on initialization
    useEffect(() => {
        const loadExtensions = async () => {
            const storedExtensions = await loadAndAddStoredExtensions(); // handles loading extensions in the backend
            console.log("12345", storedExtensions)
            setStoredExtensions(storedExtensions);
        };

        loadExtensions();
    }, []);

    // Persist extensions to localStorage
    useEffect(() => {
        localStorage.setItem(
            'user_settings',
            JSON.stringify({ extensions: storedExtensions })
        );
    }, [storedExtensions]);

    const storeExtensionConfig = (config: FullExtensionConfig) => {
        setStoredExtensions((prev) => {
            const exists = prev.some((ext) => ext.id === config.id);
            if (!exists) {
                return [...prev, config];
            }
            return prev;
        });
    };

    const toggleExtension = async (extensionId: string) => {
        setStoredExtensions((prev) => {
            const updatedExtensions = prev.map((ext) => {
                if (ext.id === extensionId) {
                    const updatedExtension = { ...ext, enabled: !ext.enabled };

                    // Notify backend
                    if (updatedExtension.enabled) {
                        addExtensionToBackend(updatedExtension); // Enable extension
                    } else {
                        removeExtensionFromBackend(updatedExtension.id); // Disable extension
                    }

                    return updatedExtension;
                }
                return ext;
            });

            // Persist to localStorage
            localStorage.setItem('user_settings', JSON.stringify({ extensions: updatedExtensions }));

            return updatedExtensions;
        });
    };

    const addExtension = async (extension: FullExtensionConfig): Promise<void> => {
        // Update state
        setStoredExtensions((prev) => {
            const updatedExtensions = [...prev, extension];

            // Persist to localStorage
            localStorage.setItem('user_settings', JSON.stringify({ extensions: updatedExtensions }));

            return updatedExtensions;
        });

        // Optionally notify backend (make async)
        try {
            await addExtensionToBackend(extension); // Replace with your backend call if needed
        } catch (error) {
            console.error('Failed to notify backend about new extension:', error);
        }
    };


    const removeExtension = async (extensionId: string) => {
        setStoredExtensions((prev) => {
            const updatedExtensions = prev.filter((ext) => ext.id !== extensionId);

            // Persist to localStorage
            localStorage.setItem('user_settings', JSON.stringify({ extensions: updatedExtensions }));

            return updatedExtensions;
        });

        // Notify backend after updating the state
        const extensionToRemove = storedExtensions.find((ext) => ext.id === extensionId);
        if (extensionToRemove) {
            try {
                await removeExtensionFromBackend(extensionToRemove.id);
            } catch (error) {
                console.error(`Error removing extension ${extensionToRemove.id} from backend:`, error);
            }
        }
    };


    return (
        <StoredExtensionsContext.Provider
            value={{
                installedExtensions: storedExtensions,
                toggleExtension,
                removeExtension,
                addExtension,
                storeExtensionConfig,
            }}
        >
            {children}
        </StoredExtensionsContext.Provider>
    );

};

export const useStoredExtensions = () => {
    const context = useContext(StoredExtensionsContext);
    if (!context) {
        throw new Error('useStoredExtensions must be used within an InstalledExtensionsProvider');
    }
    return context;
};
