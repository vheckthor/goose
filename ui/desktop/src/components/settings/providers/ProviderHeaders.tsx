import React from 'react'
import BackButton from "@/src/components/ui/BackButton";

export function Header() {
    return (
        <div className="flex items-center justify-between mb-8">
            {/* Left-hand side exit button */}
            <aside className="w-48 border-r border-gray-100 dark:border-gray-700 px-2 pt-6">
                <div className="sticky top-8">
                    <BackButton />
                </div>
            </aside>
            <div className="flex items-center gap-4">
                <h1 className="text-2xl font-semibold tracking-tight">Configure Providers</h1>
            </div>
        </div>
    )
}