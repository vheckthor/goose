import React, { useState, useEffect } from 'react';
import { useConfig } from '../../ConfigContext';
import type { ExtensionEntry } from '../../../api/types.gen';

const ExtensionsList: React.FC = () => {
    const [extensions, setExtensions] = useState<ExtensionEntry[]>([]);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const { getExtensions, toggleExtension, removeExtension } = useConfig();

    const fetchExtensions = async () => {
        setLoading(true);
        try {
            const extensionsList = await getExtensions(true); // Force refresh
            setExtensions(extensionsList);
            setError(null);
        } catch (err) {
            setError('Failed to load extensions');
            console.error('Error loading extensions:', err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchExtensions();
    }, []);

    const handleToggle = async (name: string) => {
        try {
            await toggleExtension(name);
            // Update the local state to show the change immediately
            setExtensions(prevExtensions =>
                prevExtensions.map(ext =>
                    ext.name === name ? { ...ext, enabled: !ext.enabled } : ext
                )
            );
        } catch (err) {
            setError(`Failed to toggle extension "${name}"`);
            console.error(`Error toggling extension ${name}:`, err);
        }
    };

    const handleDelete = async (extension: ExtensionEntry) => {
        const displayName = extension.name;
        console.log("deleting this extension", extension)
        // The name field might be different than the key used in the backend
        // Look at the extension object itself to determine the correct key
        const name = extension.name;

        if (window.confirm(`Are you sure you want to delete the extension "${displayName}"?`)) {
            try {
                console.log("in front end trying to remove an extension by the name of", name)
                await removeExtension(name);
                // Remove the extension from the local state
                setExtensions(prevExtensions =>
                    prevExtensions.filter(ext => ext.name !== name)
                );
                // Refresh to ensure our list matches the backend
                setTimeout(fetchExtensions, 300);
            } catch (err) {
                setError(`Failed to delete extension "${displayName}"`);
                console.error(`Error deleting extension ${displayName}:`, err);
            }
        }
    };

    const getExtensionTypeDetails = (extension: ExtensionEntry) => {
        switch (extension.type) {
            case 'sse':
                return (
                    <div className="extension-detail">
                        <p><strong>URI:</strong> {extension.uri}</p>
                        {extension.timeout !== undefined && extension.timeout !== null && (
                            <p><strong>Timeout:</strong> {extension.timeout}ms</p>
                        )}
                    </div>
                );
            case 'stdio':
                return (
                    <div className="extension-detail">
                        <p><strong>Command:</strong> {extension.cmd}</p>
                        <p><strong>Arguments:</strong> {extension.args.join(' ')}</p>
                        {extension.timeout !== undefined && extension.timeout !== null && (
                            <p><strong>Timeout:</strong> {extension.timeout}ms</p>
                        )}
                    </div>
                );
            case 'builtin':
                return (
                    <div className="extension-detail">
                        <p><strong>Built-in Extension</strong></p>
                        {extension.timeout !== undefined && extension.timeout !== null && (
                            <p><strong>Timeout:</strong> {extension.timeout}ms</p>
                        )}
                    </div>
                );
            default:
                return (
                    <div className="extension-detail">
                        <p><strong>Unknown Extension Type</strong></p>
                    </div>
                );
        }
    };

    const renderEnvironmentVariables = (extension: ExtensionEntry) => {
        if (!extension.envs || Object.keys(extension.envs).length === 0) {
            return null;
        }

        return (
            <div className="env-variables">
                <h4>Environment Variables</h4>
                <div className="env-table">
                    {Object.entries(extension.envs).map(([key, value]) => (
                        <div key={key} className="env-row">
                            <span className="env-key">{key}</span>
                            <span className="env-value">{value}</span>
                        </div>
                    ))}
                </div>
            </div>
        );
    };

    if (loading) {
        return <div className="loading">Loading extensions...</div>;
    }

    if (error) {
        return (
            <div className="error-container">
                <p className="error-message">{error}</p>
                <button onClick={fetchExtensions} className="retry-button">
                    Retry
                </button>
            </div>
        );
    }

    return (
        <div className="extensions-container">
            <div className="extensions-header">
                <h2>Installed Extensions</h2>
                <button onClick={fetchExtensions} className="refresh-button">
                    Refresh
                </button>
            </div>

            {extensions.length === 0 ? (
                <div className="no-extensions">
                    <p>No extensions installed</p>
                </div>
            ) : (
                <div className="extensions-list">
                    {extensions.map(extension => (
                        <div
                            key={extension.name}
                            className={`extension-card ${extension.enabled ? 'enabled' : 'disabled'}`}
                        >
                            <div className="extension-header">
                                <h3 className="extension-name">{extension.name}</h3>
                                <div className="extension-actions">
                                    <label className="toggle-switch">
                                        <input
                                            type="checkbox"
                                            checked={extension.enabled}
                                            onChange={() => handleToggle(extension.name)}
                                        />
                                        <span className="toggle-slider"></span>
                                    </label>
                                    <button
                                        onClick={() => handleDelete(extension)}
                                        className="delete-button"
                                        title="Delete extension"
                                    >
                                        ✕
                                    </button>
                                </div>
                            </div>

                            <div className="extension-body">
                                <div className="extension-type">
                                    <span className="type-badge">{extension.type}</span>
                                </div>
                                {getExtensionTypeDetails(extension)}
                                {renderEnvironmentVariables(extension)}
                            </div>
                        </div>
                    ))}
                </div>
            )}

            <style jsx>{`
                .extensions-container {
                    padding: 1rem;
                    max-width: 800px;
                    margin: 0 auto;
                }

                .extensions-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 1rem;
                }

                .refresh-button {
                    padding: 0.5rem 1rem;
                    background-color: #f0f0f0;
                    border: 1px solid #ddd;
                    border-radius: 4px;
                    cursor: pointer;
                }

                .refresh-button:hover {
                    background-color: #e0e0e0;
                }

                .extensions-list {
                    display: grid;
                    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
                    gap: 1rem;
                }

                .extension-card {
                    border: 1px solid #ddd;
                    border-radius: 6px;
                    padding: 1rem;
                    transition: all 0.2s ease;
                }

                .extension-card.enabled {
                    border-left: 4px solid #4caf50;
                }

                .extension-card.disabled {
                    border-left: 4px solid #f44336;
                    opacity: 0.7;
                }

                .extension-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 0.5rem;
                    border-bottom: 1px solid #eee;
                    padding-bottom: 0.5rem;
                }

                .extension-name {
                    margin: 0;
                    font-size: 1.2rem;
                }

                .extension-actions {
                    display: flex;
                    gap: 0.5rem;
                    align-items: center;
                }

                .toggle-switch {
                    position: relative;
                    display: inline-block;
                    width: 44px;
                    height: 24px;
                }

                .toggle-switch input {
                    opacity: 0;
                    width: 0;
                    height: 0;
                }

                .toggle-slider {
                    position: absolute;
                    cursor: pointer;
                    top: 0;
                    left: 0;
                    right: 0;
                    bottom: 0;
                    background-color: #ccc;
                    border-radius: 24px;
                    transition: .3s;
                }

                .toggle-slider:before {
                    position: absolute;
                    content: "";
                    height: 18px;
                    width: 18px;
                    left: 3px;
                    bottom: 3px;
                    background-color: white;
                    border-radius: 50%;
                    transition: .3s;
                }

                input:checked + .toggle-slider {
                    background-color: #4caf50;
                }

                input:checked + .toggle-slider:before {
                    transform: translateX(20px);
                }

                .delete-button {
                    background-color: transparent;
                    border: none;
                    color: #888;
                    font-size: 1rem;
                    cursor: pointer;
                    padding: 0.25rem 0.5rem;
                    border-radius: 4px;
                }

                .delete-button:hover {
                    color: #f44336;
                    background-color: #f0f0f0;
                }

                .extension-body {
                    padding-top: 0.5rem;
                }

                .extension-type {
                    margin-bottom: 0.5rem;
                }

                .type-badge {
                    background-color: #e0e0e0;
                    border-radius: 4px;
                    padding: 0.25rem 0.5rem;
                    font-size: 0.8rem;
                    text-transform: uppercase;
                }

                .extension-detail {
                    margin-top: 0.5rem;
                }

                .extension-detail p {
                    margin: 0.25rem 0;
                    font-size: 0.9rem;
                }

                .env-variables {
                    margin-top: 1rem;
                    padding-top: 0.5rem;
                    border-top: 1px dashed #eee;
                }

                .env-variables h4 {
                    margin: 0 0 0.5rem 0;
                    font-size: 0.9rem;
                }

                .env-table {
                    font-size: 0.8rem;
                }

                .env-row {
                    display: flex;
                    margin-bottom: 0.25rem;
                }

                .env-key {
                    font-weight: bold;
                    min-width: 120px;
                }

                .loading {
                    text-align: center;
                    padding: 2rem;
                    color: #666;
                }

                .error-container {
                    text-align: center;
                    padding: 2rem;
                    color: #f44336;
                }

                .retry-button {
                    margin-top: 1rem;
                    padding: 0.5rem 1rem;
                    background-color: #f0f0f0;
                    border: 1px solid #ddd;
                    border-radius: 4px;
                    cursor: pointer;
                }

                .no-extensions {
                    text-align: center;
                    padding: 2rem;
                    color: #666;
                    background-color: #f9f9f9;
                    border-radius: 6px;
                }
            `}</style>
        </div>
    );
};

export default ExtensionsList;