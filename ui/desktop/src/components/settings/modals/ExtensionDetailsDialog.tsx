import React, { useState } from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../../ui/modal';
import { Button } from '../../ui/button';
import { Extension, Key } from '../types';
import { KeyItem } from '../KeyItem';
import { KeyDialog } from './KeyDialog';
import { showToast } from '../../ui/toast';
import { Input } from '../../ui/input';
import { GearIcon } from '@radix-ui/react-icons';

interface ExtensionDetailsDialogProps {
    isOpen: boolean;
    onClose: () => void;
    extension: Extension;
    keys: Key[];
    onUpdateKeys: (keys: Key[]) => void;
    onUpdateExtension?: (extension: Extension) => void;
}

export function ExtensionDetailsDialog({ 
    isOpen, 
    onClose, 
    extension, 
    keys, 
    onUpdateKeys,
    onUpdateExtension 
}: ExtensionDetailsDialogProps) {
    const [editingKey, setEditingKey] = useState<Key | null>(null);
    const [isAddKeyOpen, setIsAddKeyOpen] = useState(false);
    const [isEditing, setIsEditing] = useState(false);
    
    const [name, setName] = useState(extension.name);
    const [description, setDescription] = useState(extension.description);
    const [command, setCommand] = useState(extension.config?.command || '');
    const [args, setArgs] = useState(extension.config?.args?.join(' ') || '');

    React.useEffect(() => {
        setName(extension.name);
        setDescription(extension.description);
        setCommand(extension.config?.command || '');
        setArgs(extension.config?.args?.join(' ') || '');
        setIsEditing(false); // Reset edit mode when extension changes
    }, [extension]);

    const handleSaveSettings = () => {
        if (!onUpdateExtension) return;

        const updatedExtension: Extension = {
            ...extension,
            name,
            description,
            config: {
                ...extension.config,
                command,
                args: args.split(' ').filter(Boolean)
            }
        };

        onUpdateExtension(updatedExtension);
    };

    const handleAddKey = (newKey: Key) => {
        onUpdateKeys([...keys, newKey]);
        // Also update the extension's relatedKeys
        if (onUpdateExtension) {
            onUpdateExtension({
                ...extension,
                relatedKeys: [...(extension.relatedKeys || []), newKey.id]
            });
        }
        setIsAddKeyOpen(false);
        showToast("Key added successfully", "success");
    };

    const handleUpdateKey = (updatedKey: Key) => {
        onUpdateKeys(keys.map(key => 
            key.id === updatedKey.id ? updatedKey : key
        ));
        setEditingKey(null);
        showToast("Key updated successfully", "success");
    };

    const handleDeleteKey = (keyToDelete: Key) => {
        onUpdateKeys(keys.filter(key => key.id !== keyToDelete.id));
        // Also update the extension's relatedKeys
        if (onUpdateExtension) {
            onUpdateExtension({
                ...extension,
                relatedKeys: (extension.relatedKeys || []).filter(id => id !== keyToDelete.id)
            });
        }
        setEditingKey(null);
        showToast("Key deleted successfully", "success");
    };

    const handleCopyKey = async (value: string) => {
        try {
            await navigator.clipboard.writeText(value);
            showToast("Key copied to clipboard", "success");
        } catch (err) {
            showToast("Failed to copy key", "error");
        }
    };

    const inputClassName = !isEditing 
        ? 'bg-gray-50 dark:bg-gray-800 cursor-not-allowed opacity-75 border-gray-200 dark:border-gray-700' 
        : '';

    return (
        <>
            <Modal open={isOpen} onOpenChange={onClose}>
                <ModalContent className="max-w-2xl max-h-[85vh] flex flex-col">
                    <div className="flex-shrink-0 flex items-center justify-between border-b p-4">
                        <ModalTitle>Extension Settings</ModalTitle>
                        <div className="flex items-center gap-2">
                            <button
                                onClick={() => setIsEditing(!isEditing)}
                                className={`p-2 rounded-full transition-colors ${
                                    isEditing 
                                        ? 'bg-indigo-100 text-indigo-600 hover:bg-indigo-200 dark:bg-indigo-900 dark:text-indigo-300' 
                                        : 'hover:bg-gray-100 dark:hover:bg-gray-700'
                                }`}
                                title={isEditing ? "Disable editing" : "Enable editing"}
                            >
                                <GearIcon className="w-5 h-5" />
                            </button>
                        </div>
                    </div>

                    <div className="flex-1 overflow-y-auto">
                        <div className="space-y-6 p-6">
                            {/* Extension Settings Section */}
                            <div className="space-y-4">
                                <div className="flex items-center justify-between">
                                    <h3 className="text-lg font-medium">Extension Configuration</h3>
                                    {!isEditing && (
                                        <span className="text-sm text-gray-500 dark:text-gray-400">
                                            Click the gear icon to edit
                                        </span>
                                    )}
                                </div>
                                <div>
                                    <label className="text-sm font-medium">Name</label>
                                    <Input
                                        value={name}
                                        onChange={(e) => setName(e.target.value)}
                                        placeholder="Extension name"
                                        readOnly={!isEditing}
                                        className={inputClassName}
                                    />
                                </div>
                                
                                <div>
                                    <label className="text-sm font-medium">Description</label>
                                    <Input
                                        value={description}
                                        onChange={(e) => setDescription(e.target.value)}
                                        placeholder="Extension description"
                                        readOnly={!isEditing}
                                        className={inputClassName}
                                    />
                                </div>

                                <div>
                                    <label className="text-sm font-medium">Command/Binary</label>
                                    <Input
                                        value={command}
                                        onChange={(e) => setCommand(e.target.value)}
                                        placeholder="e.g. /usr/local/bin/goosed or node"
                                        readOnly={!isEditing}
                                        className={inputClassName}
                                    />
                                </div>

                                <div>
                                    <label className="text-sm font-medium">Arguments</label>
                                    <Input
                                        value={args}
                                        onChange={(e) => setArgs(e.target.value)}
                                        placeholder="e.g. mcp developer"
                                        readOnly={!isEditing}
                                        className={inputClassName}
                                    />
                                </div>

                                {isEditing && (
                                    <Button 
                                        onClick={handleSaveSettings}
                                        className="w-full"
                                    >
                                        Save Settings
                                    </Button>
                                )}
                            </div>

                            {/* Keys Section */}
                            <div className="border-t pt-6">
                                <div className="flex justify-between items-center mb-4">
                                    <h3 className="text-lg font-medium">Environment Keys</h3>
                                    {isEditing && (
                                        <Button 
                                            variant="outline" 
                                            onClick={() => setIsAddKeyOpen(true)}
                                        >
                                            Add Key
                                        </Button>
                                    )}
                                </div>
                                <div className="space-y-2">
                                    {keys
                                        .filter(key => extension.relatedKeys?.includes(key.id))
                                        .map(key => (
                                            <KeyItem
                                                key={key.id}
                                                keyData={key}
                                                onEdit={isEditing ? setEditingKey : undefined}
                                                onCopy={handleCopyKey}
                                            />
                                        ))
                                    }
                                    {(!extension.relatedKeys || extension.relatedKeys.length === 0) && (
                                        <p className="text-gray-500 dark:text-gray-400 text-center py-4">
                                            No environment keys configured
                                        </p>
                                    )}
                                </div>
                            </div>
                        </div>
                    </div>
                </ModalContent>
            </Modal>

            <KeyDialog
                isOpen={isAddKeyOpen || !!editingKey}
                onClose={() => {
                    setIsAddKeyOpen(false);
                    setEditingKey(null);
                }}
                onSubmit={editingKey ? handleUpdateKey : handleAddKey}
                onDelete={handleDeleteKey}
                initialKey={editingKey || undefined}
            />
        </>
    );
} 