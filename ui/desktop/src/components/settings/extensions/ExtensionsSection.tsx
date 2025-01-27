import React from 'react';
import { ExtensionItem } from './ExtensionItem';
import { FullExtensionConfig} from "../../../extensions";

export function ExtensionsSection({
                                      extensions,
                                      onToggle,
                                      onConfigure,
                                      onAddManualExtension,
                                  }: {
    extensions: FullExtensionConfig[];
    onToggle: (id: string) => void;
    onConfigure: (extension: FullExtensionConfig) => void;
    onAddManualExtension: () => void; // Add this to the type
}) {
    return (
        <section id="extensions">
            <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
                <h2 className="text-xl font-semibold text-textStandard">Extensions</h2>
                <div className="flex gap-4">
                    <button
                        onClick={onAddManualExtension}
                        className="text-indigo-500 hover:text-indigo-600 text-sm"
                    >
                        Add
                    </button>
                    <button
                        onClick={() => window.electron.openInChrome('https://block.github.io/goose/v1/extensions/')}
                        className="text-indigo-500 hover:text-indigo-600 text-sm"
                    >
                        Browse
                    </button>
                </div>
            </div>

            <div className="px-8">
                {extensions.length === 0 ? (
                    <p className="text-textSubtle text-center py-4">No Extensions Added</p>
                ) : (
                    extensions.map((ext) => (
                        <ExtensionItem
                            key={ext.id}
                            {...ext}
                            canConfigure={true}
                            onToggle={onToggle}
                            onConfigure={() => onConfigure(ext)}
                        />
                    ))
                )}
            </div>
        </section>
    );
}
