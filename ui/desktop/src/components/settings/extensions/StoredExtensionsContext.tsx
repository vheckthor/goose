import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import {FullExtensionConfig, BUILT_IN_EXTENSIONS, loadAndAddStoredExtensions} from '../../../extensions';
import {addExtension as addExtensionToBackend} from "../../../extensions";
import {removeExtension as removeExtensionFromBackend} from "../../../extensions";
import {getStoredExtensionsAndBuiltIns} from "./utils";

const StoredExtensionsContext = createContext<
    | {
    storedExtensions: FullExtensionConfig[];
    toggleExtension: (extension: string) => void;
    removeExtension: (extension: string) => void;
    addExtension: (extension: FullExtensionConfig) => Promise<void>;
    storeExtensionConfig: (extension: FullExtensionConfig) => void;
}
    | undefined
>(undefined);

export const StoredExtensionsProvider = ({ children }: { children: ReactNode }) => {
    const [storedExtensions, setStoredExtensions] = useState<FullExtensionConfig[]>([]);
    // Get extensions from local storage / builtins on initialization
    useEffect(() => {
        const loadExtensions = async () => {
            const storedExtensions = await getStoredExtensionsAndBuiltIns();
            setStoredExtensions(storedExtensions);
        };

        loadExtensions();
    }, []);

    // responsible for writing storedExtensions to localStorage whenever the state changes
    useEffect(() => {
        localStorage.setItem(
            'user_settings',
            JSON.stringify({ extensions: storedExtensions })
        );
    }, [storedExtensions]);

    // function to add a single extension configuration (config) to the storedExtensions array only if it doesn’t already exist
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
        setStoredExtensions((prev) =>
            prev.map((ext) => {
                if (ext.id === extensionId) {
                    const updatedExtension = { ...ext, enabled: !ext.enabled };

                    // Notify backend asynchronously
                    if (updatedExtension.enabled) {
                        addExtensionToBackend(updatedExtension).catch((error) => {
                            console.error(`Failed to enable extension ${updatedExtension.name} in backend:`, error);
                        });
                    } else {
                        removeExtensionFromBackend(updatedExtension.id).catch((error) => {
                            console.error(`Failed to disable extension ${updatedExtension.name} in backend:`, error);
                        });
                    }

                    return updatedExtension; // Update the state with the toggled extension
                }
                return ext; // Return unchanged extensions
            })
        );
    };

    // Add a new extension (with duplicate prevention)
    const addExtension = async (extension: FullExtensionConfig): Promise<void> => {
        setStoredExtensions((prev) => {
            // Prevent duplicates by checking for the same `id`
            const exists = prev.some((ext) => ext.id === extension.id);
            if (!exists) {
                return [...prev, extension]; // Add the extension if it doesn't exist
            }
            return prev; // Return the existing state if the extension already exists
        });

        // Notify backend asynchronously
        try {
            await addExtensionToBackend(extension); // Replace with your backend call
        } catch (error) {
            console.error('Failed to notify backend about new extension:', error);
        }
    };

    const removeExtension = async (extensionId: string) => {
        setStoredExtensions((prev) => {
            const updatedExtensions = prev.filter((ext) => ext.id !== extensionId);

            // Notify backend about the removal
            const extensionToRemove = prev.find((ext) => ext.id === extensionId);
            if (extensionToRemove) {
                removeExtensionFromBackend(extensionToRemove.id)
                    .catch((error) => {
                        console.error(`Error removing extension ${extensionToRemove.id} from backend:`, error);
                    });
            }

            return updatedExtensions; // Update the state
        });
    };


    return (
        <StoredExtensionsContext.Provider
            value={{
                storedExtensions: storedExtensions,
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
