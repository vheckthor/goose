import React from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../ui/dialog';
import { Button } from '../../ui/button';
import { Key } from '../types';

interface RevealKeysDialogProps {
    isOpen: boolean;
    onClose: () => void;
    keys: Key[];
}

export function RevealKeysDialog({ isOpen, onClose, keys }: RevealKeysDialogProps) {
    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>All Keys (Development Only)</DialogTitle>
                </DialogHeader>
                <div className="space-y-4 max-h-[60vh] overflow-y-auto">
                    {keys.map(key => (
                        <div key={key.id} className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                            <div className="font-medium text-sm text-gray-700 dark:text-gray-300">
                                {key.name}
                            </div>
                            <div className="mt-1 font-mono text-sm text-gray-600 dark:text-gray-400 break-all">
                                {key.value}
                            </div>
                        </div>
                    ))}
                </div>
                <div className="flex justify-end">
                    <Button variant="outline" onClick={onClose}>Close</Button>
                </div>
            </DialogContent>
        </Dialog>
    );
} 