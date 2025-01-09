import React, { useState } from 'react';
import { Key } from './types';

interface KeyItemProps {
    keyData: Key;
    onEdit: (key: Key) => void;
    onCopy: (value: string) => void;
}

export function KeyItem({ keyData, onEdit, onCopy }: KeyItemProps) {
    const [isValueVisible, setIsValueVisible] = useState(false);

    const handleCopy = () => {
        onCopy(keyData.value);
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
            <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium dark:text-white">{keyData.name}</h3>
                <div className="flex items-center gap-3">
                    <div className="flex items-center">
                        <span className="text-gray-500">
                            {isValueVisible ? keyData.value : '*'.repeat(17)}
                        </span>
                        <button
                            onClick={() => setIsValueVisible(!isValueVisible)}
                            className="ml-2 text-gray-400 hover:text-gray-600"
                        >
                            {isValueVisible ? '👁️' : '👁️‍🗨️'}
                        </button>
                        <button
                            onClick={handleCopy}
                            className="ml-2 text-indigo-500 hover:text-indigo-600"
                        >
                            📋
                        </button>
                    </div>
                    <button 
                        onClick={() => onEdit(keyData)}
                        className="text-indigo-500 hover:text-indigo-600"
                    >
                        ✏️
                    </button>
                </div>
            </div>
        </div>
    );
} 