import React, { createContext, useContext, useState, ReactNode, useEffect } from 'react';
//import { getExtensions } from './utils';
import { FullExtensionConfig } from "../../../extensions";

// Create a context for active keys
const InstalledExtensionsContext = createContext<
    | {
    installedExtensions: FullExtensionConfig[];
    setInstalledExtensions: (extensions: FullExtensionConfig[]) => void;
}
    | undefined
>(undefined);

export const InstalledExtensionsProvider = ({ children }: { children: ReactNode }) => {
    const [installedExtensions, setInstalledExtensions] = useState<FullExtensionConfig[]>([]); // Start with an empty list
    const [isLoading, setIsLoading] = useState(true); // Track loading state

    // Fetch settings
    useEffect(() => {
        const fetchInstalledExtensions = async () => {
            try {
                const extensions = JSON.parse(localStorage.getItem('user_settings')) || 'null' // Fetch the installed extensions
                setInstalledExtensions(extensions); // Update state with fetched extensions
            } catch (error) {
                console.error('Error fetching installed extensions:', error);
            } finally {
                setIsLoading(false); // Ensure loading is marked as complete
            }
        };

        fetchInstalledExtensions(); // Call the async function
    }, []);

    console.log("installed extensions", installedExtensions)
    // Provide active keys and ability to update them
    return (
        <InstalledExtensionsContext.Provider value={{ installedExtensions, setInstalledExtensions }}>
            {!isLoading ? children : <div>Loading...</div>} {/* Conditional rendering */}
        </InstalledExtensionsContext.Provider>
    );
};

// Custom hook to access installed extensions
export const useInstalledExtensions = () => {
    const context = useContext(InstalledExtensionsContext);
    if (!context) {
        throw new Error('useInstalledExtensions must be used within an InstalledExtensionsProvider');
    }
    return context;
};
