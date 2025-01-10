import React from 'react';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '../../ui/dialog';
import { Button } from '../../ui/button';

interface BaseDialogProps {
    title: string;
    isOpen: boolean;
    onClose: () => void;
    onSubmit?: (e: React.FormEvent) => void;
    children: React.ReactNode;
}

export function BaseDialog({ title, isOpen, onClose, onSubmit, children }: BaseDialogProps) {
    const isEditing = title.startsWith('Edit');
    
    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{title}</DialogTitle>
                </DialogHeader>
                {children}
                <div className="flex justify-end gap-2">
                    <Button type="button" variant="outline" onClick={onClose}>
                        Cancel
                    </Button>
                    <Button type="submit" form="keyForm">
                        {isEditing ? 'Save Changes' : 'Add'}
                    </Button>
                </div>
            </DialogContent>
        </Dialog>
    );
} 