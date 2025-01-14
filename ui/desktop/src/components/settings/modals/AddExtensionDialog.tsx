import React, { useState } from 'react';
import { BaseDialog } from './BaseDialog';
import { Input } from '../../ui/input';
import { Textarea } from '../../ui/textarea';
import { showToast } from '../../ui/toast';
import { Extension } from '../types';

interface AddExtensionDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onAdd: (extension: Extension) => void;
}

export function AddExtensionDialog({ isOpen, onClose, onAdd }: AddExtensionDialogProps) {
    const [command, setCommand] = useState('');
    const [argString, setArgString] = useState('');
    const [envString, setEnvString] = useState('');
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');

    // Reset form when dialog closes
    React.useEffect(() => {
        if (!isOpen) {
            setCommand('');
            setArgString('');
            setEnvString('');
            setName('');
            setDescription('');
        }
    }, [isOpen]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        // Split arguments by space
        const argsArray = argString
            .split(' ')
            .map((arg) => arg.trim())
            .filter(Boolean);

        // Parse env lines into an object
        const envObject: Record<string, string> = {};
        envString.split('\n').forEach((line) => {
            const [key, ...rest] = line.split('=');
            if (key && rest.length) {
                envObject[key.trim()] = rest.join('=').trim();
            }
        });

        const payload = {
            type: 'Stdio',
            cmd: command.trim(),
            args: argsArray,
            env: envObject,
        };

        try {
            const response = await fetch('http://localhost:53920/systems/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
            });

            if (!response.ok) {
                throw new Error(`Failed to add server: ${response.statusText}`);
            }

            // Create the extension object
            const extension: Extension = {
                id: name.toLowerCase().replace(/\s+/g, '-'),
                name,
                description,
                enabled: true,
                config: {
                    command,
                    args: argsArray,
                    env: envObject
                }
            };

            onAdd(extension);
            showToast("Extension added successfully", "success");
            onClose();
        } catch (err) {
            showToast(err instanceof Error ? err.message : "Failed to add extension", "error");
        }
    };

    return (
        <BaseDialog 
            title="Add Extension" 
            isOpen={isOpen} 
            onClose={onClose}
            onSubmit={handleSubmit}
        >
            <form id="keyForm" onSubmit={handleSubmit} className="space-y-4">
                <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder="Extension name"
                        required
                    />
                </div>
                
                <div>
                    <label className="text-sm font-medium">Description</label>
                    <Input
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="Extension description"
                    />
                </div>

                <div>
                    <label className="text-sm font-medium">Command/Binary</label>
                    <Input
                        value={command}
                        onChange={(e) => setCommand(e.target.value)}
                        placeholder="e.g. /usr/local/bin/goosed or node"
                        required
                    />
                </div>

                <div>
                    <label className="text-sm font-medium">Arguments</label>
                    <Input
                        value={argString}
                        onChange={(e) => setArgString(e.target.value)}
                        placeholder="e.g. mcp developer"
                    />
                </div>

                <div>
                    <label className="text-sm font-medium">Environment Variables</label>
                    <Textarea
                        value={envString}
                        onChange={(e) => setEnvString(e.target.value)}
                        placeholder={`Enter key=value pairs.\nFor example:\nMY_API_KEY=my-secret-key\nFOO=bar`}
                        rows={4}
                    />
                </div>
            </form>
        </BaseDialog>
    );
} 