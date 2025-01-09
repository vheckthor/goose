import React from 'react';
import { BaseDialog } from './BaseDialog';
import { Key } from '../types';
import { Input } from '../../ui/input';

interface KeyDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: (key: Key) => void;
    initialKey?: Key;
}

export function KeyDialog({ isOpen, onClose, onSubmit, initialKey }: KeyDialogProps) {
    const [name, setName] = React.useState(initialKey?.name || '');
    const [value, setValue] = React.useState(initialKey?.value || '');
    const isEditing = !!initialKey;

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        const id = initialKey?.id || name.toLowerCase().replace(/\s+/g, '-');
        onSubmit({
            id,
            name,
            value
        });
        onClose();
        if (!isEditing) {
            setName('');
            setValue('');
        }
    };

    return (
        <BaseDialog 
            title={isEditing ? 'Edit Key' : 'Add Key'} 
            isOpen={isOpen} 
            onClose={onClose}
        >
            <form onSubmit={handleSubmit} className="space-y-4">
                <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder="Key name"
                        required
                        disabled={isEditing}
                    />
                </div>
                <div>
                    <label className="text-sm font-medium">Value</label>
                    <Input
                        type="password"
                        value={value}
                        onChange={(e) => setValue(e.target.value)}
                        placeholder="Key value"
                        required
                    />
                </div>
            </form>
        </BaseDialog>
    );
} 