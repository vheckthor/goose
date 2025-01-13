import React, { useState } from 'react';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { useNavigate } from 'react-router-dom';
import { Settings as SettingsType, Model, Extension, Key } from './types';
import { ToggleableItem } from './ToggleableItem';
import { KeyItem } from './KeyItem';
import { AddModelDialog } from './modals/AddModelDialog';
import { KeyDialog } from './modals/KeyDialog';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../ui/modal';
import { Button } from '../ui/button';
import { RevealKeysDialog } from './modals/RevealKeysDialog';
import { showToast } from '../ui/toast';

<<<<<<< HEAD
const EXTENSIONS_DESCRIPTION = "The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.";

const DEFAULT_SETTINGS: SettingsType = {
    models: [
        { id: "gpt4", name: "GPT 4.0", description: "Standard config", enabled: false },
        { id: "gpt4lite", name: "GPT 4.0 lite", description: "Standard config", enabled: false },
        { id: "claude", name: "Claude", description: "Standard config", enabled: true }
    ],
    extensions: [
        { id: "fileviewer", name: "File viewer", description: "Standard config", enabled: true },
        { id: "cloudthing", name: "Cloud thing", description: "Standard config", enabled: true },
        { id: "mcpdice", name: "MCP dice", description: "Standard config", enabled: true },
        { id: "binancedata", name: "Binance market data", description: "Standard config", enabled: true }
    ],
    keys: [
        { id: "giskey", name: "GISKey", value: "*****************" },
        { id: "awscognito", name: "AWScognito", value: "*****************" }
    ]
};
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
const MCP_DESCRIPTION = "The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.";
=======
const EXTENSIONS_DESCRIPTION = "The Model Context Protocol (MCP) is a system that allows AI models to securely connect with local or remote resources using standard server setups. It works like a client-server setup and expands AI capabilities using three main components: Prompts, Resources, and Tools.";
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)

export default function Settings() {
    const navigate = useNavigate();
<<<<<<< HEAD
    
    const [settings, setSettings] = React.useState<SettingsType>(() => {
        const saved = localStorage.getItem('user_settings');
        return saved ? JSON.parse(saved) : DEFAULT_SETTINGS;
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
    const [models, setModels] = React.useState({
        gpt4: false,
        gpt4lite: false,
        claude: true,
    });
    const [mcps, setMcps] = React.useState({
        fileviewer: true,
        cloudthing: true,
        mcpdice: true,
        binancedata: true,
=======
    
    // Initialize models state from localStorage or use default values
    const [models, setModels] = React.useState(() => {
        const savedModels = localStorage.getItem('settings_models');
        return savedModels ? JSON.parse(savedModels) : {
            gpt4: false,
            gpt4lite: false,
            claude: true,
        };
    });

    // Initialize extensions state from localStorage or use default values
    const [extensions, setExtensions] = React.useState(() => {
        const savedExtensions = localStorage.getItem('settings_extensions');
        return savedExtensions ? JSON.parse(savedExtensions) : {
            fileviewer: true,
            cloudthing: true,
            mcpdice: true,
            binancedata: true,
        };
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
    });

<<<<<<< HEAD
    // Persist settings changes
    React.useEffect(() => {
        localStorage.setItem('user_settings', JSON.stringify(settings));
    }, [settings]);

||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
=======
    // Save models state to localStorage whenever it changes
    React.useEffect(() => {
        localStorage.setItem('settings_models', JSON.stringify(models));
    }, [models]);

    // Save extensions state to localStorage whenever it changes
    React.useEffect(() => {
        localStorage.setItem('settings_extensions', JSON.stringify(extensions));
    }, [extensions]);

>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
    const handleModelToggle = (modelId: string) => {
        setSettings(prev => ({
            ...prev,
            models: prev.models.map(model => ({
                ...model,
                enabled: model.id === modelId
            }))
        }));
    };

<<<<<<< HEAD
    const handleExtensionToggle = (extensionId: string) => {
        setSettings(prev => ({
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
    const handleMcpToggle = (mcpId: string) => {
        setMcps(prev => ({
=======
    const handleExtensionToggle = (extensionId: string) => {
        setExtensions(prev => ({
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
            ...prev,
<<<<<<< HEAD
            extensions: prev.extensions.map(ext => 
                ext.id === extensionId ? { ...ext, enabled: !ext.enabled } : ext
            )
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
            [mcpId]: !prev[mcpId]
=======
            [extensionId]: !prev[extensionId]
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
        }));
    };

<<<<<<< HEAD
    const handleNavClick = (section: string, e: React.MouseEvent) => {
        e.preventDefault();
        const scrollArea = document.querySelector('[data-radix-scroll-area-viewport]');
        const element = document.getElementById(section.toLowerCase());
        
        if (scrollArea && element) {
            const topPos = element.offsetTop;
            scrollArea.scrollTo({
                top: topPos,
                behavior: 'smooth'
            });
        }
    };

    const handleExit = () => {
        navigate('/chat/1', { replace: true }); // Use replace to ensure clean navigation
    };

    const [addModelOpen, setAddModelOpen] = useState(false);
    const [addKeyOpen, setAddKeyOpen] = useState(false);
    const [editingKey, setEditingKey] = useState<Key | null>(null);
    const [showResetConfirm, setShowResetConfirm] = useState(false);
    const [showAllKeys, setShowAllKeys] = useState(false);

    const handleAddModel = (newModel: Model) => {
        setSettings(prev => ({
            ...prev,
            models: [...prev.models, { ...newModel, enabled: false }]
        }));
        setAddModelOpen(false);
    };

    const handleAddKey = (newKey: Key) => {
        setSettings(prev => ({
            ...prev,
            keys: [...prev.keys, newKey]
        }));
        setAddKeyOpen(false);
    };

    const handleUpdateKey = (updatedKey: Key) => {
        setSettings(prev => ({
            ...prev,
            keys: prev.keys.map(key => 
                key.id === updatedKey.id ? updatedKey : key
            )
        }));
        setEditingKey(null);
    };

    const handleCopyKey = async (value: string) => {
        try {
            await navigator.clipboard.writeText(value);
            // Could add a toast notification here
        } catch (err) {
            console.error('Failed to copy:', err);
        }
    };

    const handleDeleteKey = (keyToDelete: Key) => {
        setSettings(prev => ({
            ...prev,
            keys: prev.keys.filter(key => key.id !== keyToDelete.id)
        }));
        setEditingKey(null);
    };

    const handleReset = () => {
        setSettings(DEFAULT_SETTINGS);
        setShowResetConfirm(false);
        showToast("Settings reset to default", "success");
    };

||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
=======
    const handleNavClick = (section: string, e: React.MouseEvent) => {
        e.preventDefault();
        const scrollArea = document.querySelector('[data-radix-scroll-area-viewport]');
        const element = document.getElementById(section.toLowerCase());
        
        if (scrollArea && element) {
            const topPos = element.offsetTop;
            scrollArea.scrollTo({
                top: topPos,
                behavior: 'smooth'
            });
        }
    };

    const handleExit = () => {
        navigate('/chat/1', { replace: true }); // Use replace to ensure clean navigation
    };

>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
    return (
        <div className="h-screen w-full p-[10px]">
            <Card className="h-full w-full bg-card-gradient dark:bg-dark-card-gradient border-none rounded-2xl p-6">
                <div className="h-full w-full bg-white dark:bg-gray-800 rounded-2xl overflow-hidden p-4">
                    <ScrollArea className="h-full w-full">
                        <div className="flex min-h-full">
                            {/* Left Navigation */}
                            <div className="w-48 border-r border-gray-100 dark:border-gray-700 px-6">
                                <div className="sticky top-8">
                                    <button
                                        onClick={handleExit}
                                        className="flex items-center gap-2 text-gray-600 hover:text-gray-800 
                                            dark:text-gray-400 dark:hover:text-gray-200 mb-16 mt-4"
                                    >
                                        <span className="text-xl">←</span>
                                        <span>Exit</span>
                                    </button>
                                    <div className="space-y-2">
                                        {['Models', 'Extensions', 'Keys'].map((section) => (
                                            <button
                                                key={section}
                                                onClick={(e) => handleNavClick(section, e)}
                                                className="block w-full text-left px-3 py-2 rounded-lg transition-colors
                                                    text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                                            >
                                                {section}
                                            </button>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            {/* Content Area */}
                            <div className="flex-1 px-8 py-8">
                                <div className="max-w-3xl space-y-12">
                                    {/* Models Section */}
                                    <section id="models">
                                        <div className="flex justify-between items-center mb-4">
                                            <h2 className="text-2xl font-semibold">Models</h2>
                                            <button 
                                                onClick={() => setAddModelOpen(true)}
                                                className="text-indigo-500 hover:text-indigo-600 font-medium"
                                            >
                                                Add Models
                                            </button>
                                        </div>
                                        {settings.models.map(model => (
                                            <ToggleableItem
                                                key={model.id}
                                                {...model}
                                                onToggle={handleModelToggle}
                                            />
                                        ))}
                                    </section>

<<<<<<< HEAD
                                    {/* Extensions Section */}
                                    <section id="extensions">
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                    {/* MCPs Section */}
                                    <section id="mcps">
=======
                                    {/* Extensions Section (formerly MCPs) */}
                                    <section id="extensions">
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        <div className="flex justify-between items-center mb-4">
<<<<<<< HEAD
                                            <h2 className="text-2xl font-semibold">Extensions</h2>
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                            <h2 className="text-2xl font-semibold">MCPs</h2>
                                            <button className="text-indigo-500 hover:text-indigo-600 font-medium">
                                                Add MCPs
                                            </button>
=======
                                            <h2 className="text-2xl font-semibold">Extensions</h2>
                                            <button className="text-indigo-500 hover:text-indigo-600 font-medium">
                                                Add Extensions
                                            </button>
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        </div>
<<<<<<< HEAD
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {settings.extensions.map(ext => (
                                            <ToggleableItem
                                                key={ext.id}
                                                {...ext}
                                                onToggle={handleExtensionToggle}
                                            />
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{MCP_DESCRIPTION}</p>
                                        {Object.entries(mcps).map(([id, enabled]) => (
                                            <div key={id} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">
                                                        {id === 'fileviewer' ? 'File viewer' :
                                                         id === 'cloudthing' ? 'Cloud thing' :
                                                         id === 'mcpdice' ? 'MCP dice' : 'Binance market data'}
                                                    </h3>
                                                    <button 
                                                        onClick={() => handleMcpToggle(id)}
                                                        className={`
                                                            relative inline-flex h-6 w-11 items-center rounded-full
                                                            ${enabled ? 'bg-indigo-500' : 'bg-gray-200 dark:bg-gray-600'}
                                                            transition-colors duration-200 ease-in-out focus:outline-none
                                                        `}
                                                    >
                                                        <span
                                                            className={`
                                                                inline-block h-5 w-5 transform rounded-full bg-white shadow
                                                                transition-transform duration-200 ease-in-out
                                                                ${enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'}
                                                            `}
                                                        />
                                                    </button>
                                                </div>
                                                <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">
                                                    Standard config
                                                </p>
                                            </div>
=======
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {Object.entries(extensions).map(([id, enabled]) => (
                                            <div key={id} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">
                                                        {id === 'fileviewer' ? 'File viewer' :
                                                         id === 'cloudthing' ? 'Cloud thing' :
                                                         id === 'mcpdice' ? 'MCP dice' : 'Binance market data'}
                                                    </h3>
                                                    <button 
                                                        onClick={() => handleExtensionToggle(id)}
                                                        className={`
                                                            relative inline-flex h-6 w-11 items-center rounded-full
                                                            ${enabled ? 'bg-indigo-500' : 'bg-gray-200 dark:bg-gray-600'}
                                                            transition-colors duration-200 ease-in-out focus:outline-none
                                                        `}
                                                    >
                                                        <span
                                                            className={`
                                                                inline-block h-5 w-5 transform rounded-full bg-white shadow
                                                                transition-transform duration-200 ease-in-out
                                                                ${enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'}
                                                            `}
                                                        />
                                                    </button>
                                                </div>
                                                <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">
                                                    Standard config
                                                </p>
                                            </div>
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        ))}
                                    </section>

                                    {/* Keys Section */}
                                    <section id="keys">
                                        <div className="flex justify-between items-center mb-4">
                                            <h2 className="text-2xl font-semibold">Keys</h2>
                                            <button 
                                                onClick={() => setAddKeyOpen(true)}
                                                className="text-indigo-500 hover:text-indigo-600 font-medium"
                                            >
                                                Add new key
                                            </button>
                                        </div>
<<<<<<< HEAD
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {settings.keys.map(keyItem => (
                                            <KeyItem
                                                key={keyItem.id}
                                                keyData={keyItem}
                                                onEdit={setEditingKey}
                                                onCopy={handleCopyKey}
                                            />
||||||| parent of ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{MCP_DESCRIPTION}</p>
                                        {['GISKey', 'AWScognito'].map(key => (
                                            <div key={key} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">{key}</h3>
                                                    <div className="flex items-center gap-3">
                                                        <span className="text-gray-500">{'*'.repeat(17)}</span>
                                                        <button className="text-indigo-500 hover:text-indigo-600">
                                                            ✏️
                                                        </button>
                                                    </div>
                                                </div>
                                            </div>
=======
                                        <p className="text-gray-500 dark:text-gray-400 mb-4">{EXTENSIONS_DESCRIPTION}</p>
                                        {['GISKey', 'AWScognito'].map(key => (
                                            <div key={key} className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
                                                <div className="flex justify-between items-center">
                                                    <h3 className="text-lg font-medium dark:text-white">{key}</h3>
                                                    <div className="flex items-center gap-3">
                                                        <span className="text-gray-500">{'*'.repeat(17)}</span>
                                                        <button className="text-indigo-500 hover:text-indigo-600">
                                                            ✏️
                                                        </button>
                                                    </div>
                                                </div>
                                            </div>
>>>>>>> ea1dc08d (fix: resolve merge conflicts in router imports and navigation)
                                        ))}

                                        <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                                            <Button
                                                variant="outline"
                                                onClick={() => setShowAllKeys(true)}
                                                className="w-full text-yellow-600 hover:text-yellow-700 dark:text-yellow-500 dark:hover:text-yellow-400"
                                            >
                                                Reveal All Keys (Dev Only)
                                            </Button>
                                        </div>
                                    </section>

                                    {/* Reset Button */}
                                    <div className="pt-8 border-t border-gray-200 dark:border-gray-700">
                                        <Button
                                            onClick={() => setShowResetConfirm(true)}
                                            variant="destructive"
                                            className="w-full"
                                        >
                                            Reset to Default Settings
                                        </Button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </ScrollArea>
                </div>
            </Card>

            {/* Reset Confirmation Dialog */}
            <Modal open={showResetConfirm} onOpenChange={setShowResetConfirm}>
                <ModalContent>
                    <ModalHeader>
                        <ModalTitle>Reset Settings</ModalTitle>
                    </ModalHeader>
                    <div className="py-4">
                        <p className="text-gray-600 dark:text-gray-300">
                            Are you sure you want to reset all settings to their default values? This cannot be undone.
                        </p>
                    </div>
                    <div className="flex justify-end gap-2">
                        <Button
                            variant="outline"
                            onClick={() => setShowResetConfirm(false)}
                        >
                            Cancel
                        </Button>
                        <Button
                            variant="destructive"
                            onClick={handleReset}
                        >
                            Reset Settings
                        </Button>
                    </div>
                </ModalContent>
            </Modal>

            {/* Add the modals */}
            <AddModelDialog
                isOpen={addModelOpen}
                onClose={() => setAddModelOpen(false)}
                onAdd={handleAddModel}
            />
            <KeyDialog
                isOpen={addKeyOpen || !!editingKey}
                onClose={() => {
                    setAddKeyOpen(false);
                    setEditingKey(null);
                }}
                onSubmit={editingKey ? handleUpdateKey : handleAddKey}
                onDelete={handleDeleteKey}
                initialKey={editingKey || undefined}
            />

            <RevealKeysDialog
                isOpen={showAllKeys}
                onClose={() => setShowAllKeys(false)}
                keys={settings.keys}
            />
        </div>
    );
} 