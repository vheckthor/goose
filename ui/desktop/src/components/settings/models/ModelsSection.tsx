import React from 'react';
import { RecentModelsRadio } from './RecentModels';

export function ModelsSection({ onBrowse }: { onBrowse: () => void }) {
    return (
        <section id="models">
            <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
                <h2 className="text-xl font-medium text-textStandard">Models</h2>
                <button
                    onClick={onBrowse}
                    className="text-indigo-500 hover:text-indigo-600 text-sm"
                >
                    Browse
                </button>
            </div>
            <div className="px-8">
                <RecentModelsRadio />
            </div>
        </section>
    );
}
