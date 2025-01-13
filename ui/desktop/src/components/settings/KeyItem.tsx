import React from 'react';

export interface KeyItemProps {
    name: string;
    value: string;
    onEdit: () => void;
}

export function KeyItem({ name, value, onEdit }: KeyItemProps) {
    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
            <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium dark:text-white">{name}</h3>
                <div className="flex items-center gap-3">
                    <span className="text-gray-500">{value}</span>
                    <button 
                        onClick={onEdit}
                        className="text-indigo-500 hover:text-indigo-600"
                    >
                        ✏️
                    </button>
                </div>
            </div>
        </div>
    );
} 