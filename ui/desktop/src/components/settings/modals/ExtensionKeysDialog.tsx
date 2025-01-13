import React, { useState } from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../../ui/modal';
import { Button } from '../../ui/button';
import { Extension, Key } from '../types';
import { KeyItem } from '../KeyItem';
import { KeyDialog } from './KeyDialog';
import { showToast } from '../../ui/toast';

interface ExtensionKeysDialogProps {
    isOpen: boolean;
    onClose: () => void;
    extension: Extension;
    keys: Key[];
    onUpdateKeys: (keys: Key[]) => void;
}

export function ExtensionKeysDialog({ 
    isOpen, 
    onClose, 
    extension, 
    keys, 
    onUpdateKeys 
}: ExtensionKeysDialogProps) {
    const [editingKey, setEditingKey] = useState<Key | null>(null);
    const [isAddKeyOpen, setIsAddKeyOpen] = useState(false);

    const handleAddKey = (newKey: Key) => {
        onUpdateKeys([...keys, newKey]);
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

    return (
        <>
            <Modal open={isOpen} onOpenChange={onClose}>
                <ModalContent className="max-w-2xl">
                    <ModalHeader>
                        <ModalTitle>{extension.name} Configuration</ModalTitle>
                    </ModalHeader>
                    <div className="py-4">
                        <div className="mb-4">
                            <h3 className="text-lg font-medium mb-2">Description</h3>
                            <p className="text-gray-600 dark:text-gray-300">
                                {extension.description}
                            </p>
                        </div>
                        
                        <div className="mb-4">
                            <div className="flex justify-between items-center mb-2">
                                <h3 className="text-lg font-medium">Environment Variables</h3>
                                <Button 
                                    variant="outline" 
                                    onClick={() => setIsAddKeyOpen(true)}
                                >
                                    Add Key
                                </Button>
                            </div>
                            <div className="space-y-2">
                                {keys.map(key => (
                                    <KeyItem
                                        key={key.id}
                                        keyData={key}
                                        onEdit={setEditingKey}
                                        onCopy={handleCopyKey}
                                    />
                                ))}
                                {keys.length === 0 && (
                                    <p className="text-gray-500 dark:text-gray-400 text-center py-4">
                                        No environment variables configured
                                    </p>
                                )}
                            </div>
                        </div>
                    </div>
                    <div className="flex justify-end">
                        <Button variant="outline" onClick={onClose}>
                            Close
                        </Button>
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