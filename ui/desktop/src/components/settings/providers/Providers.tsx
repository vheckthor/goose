"use client"

import * as React from "react"
import { Check, ChevronDown, Edit2, Plus, X } from "lucide-react"
import { Button } from "../../ui/button"
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@radix-ui/react-accordion"
import { AddKeyModal } from "./add-key-modal"

interface Provider {
    id: string
    name: string
    keyName: string
    isConfigured: boolean
    description: string
}

const providers: Provider[] = [
    {
        id: "openai",
        name: "OpenAI",
        keyName: "OPENAI_API_KEY",
        isConfigured: true,
        description: "Access GPT-4, GPT-3.5 Turbo, and other OpenAI models",
    },
    {
        id: "anthropic",
        name: "Anthropic",
        keyName: "ANTHROPIC_API_KEY",
        isConfigured: false,
        description: "Access Claude and other Anthropic models",
    },
    {
        id: "google",
        name: "Google AI",
        keyName: "GOOGLE_API_KEY",
        isConfigured: false,
        description: "Access Gemini and other Google AI models",
    },
    {
        id: "mistral",
        name: "Mistral AI",
        keyName: "MISTRAL_API_KEY",
        isConfigured: true,
        description: "Access Mistral's large language models",
    },
]

export function Providers() {
    const [selectedProvider, setSelectedProvider] = React.useState<Provider | null>(null)
    const [isModalOpen, setIsModalOpen] = React.useState(false)

    return (
        <>
            <Accordion type="single" collapsible className="w-full space-y-4">
                {providers.map((provider) => (
                    <AccordionItem key={provider.id} value={provider.id} className="border rounded-lg px-6">
                        <AccordionTrigger className="hover:no-underline">
                            <div className="flex items-center justify-between w-full">
                                <div className="flex items-center gap-4">
                                    <div className="font-semibold">{provider.name}</div>
                                    {provider.isConfigured ? (
                                        <div className="flex items-center gap-1 text-sm text-green-600 dark:text-green-500">
                                            <Check className="h-4 w-4" />
                                            <span>Configured</span>
                                        </div>
                                    ) : (
                                        <div className="flex items-center gap-1 text-sm text-destructive">
                                            <X className="h-4 w-4" />
                                            <span>Not Configured</span>
                                        </div>
                                    )}
                                </div>
                                <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform duration-200" />
                            </div>
                        </AccordionTrigger>
                        <AccordionContent className="pt-4 pb-2">
                            <div className="space-y-4">
                                <p className="text-sm text-muted-foreground">{provider.description}</p>
                                <div className="flex items-center justify-between">
                                    <div className="text-sm">
                                        <span className="text-muted-foreground">API Key Name: </span>
                                        <code className="font-mono">{provider.keyName}</code>
                                    </div>
                                    {provider.isConfigured ? (
                                        <div className="space-x-2">
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={() => {
                                                    setSelectedProvider(provider)
                                                    setIsModalOpen(true)
                                                }}
                                            >
                                                <Edit2 className="h-4 w-4 mr-2" />
                                                Edit Key
                                            </Button>
                                            <Button
                                                variant="destructive"
                                                size="sm"
                                                onClick={() => {
                                                    // Handle delete
                                                }}
                                            >
                                                Delete Key
                                            </Button>
                                        </div>
                                    ) : (
                                        <Button
                                            size="sm"
                                            onClick={() => {
                                                setSelectedProvider(provider)
                                                setIsModalOpen(true)
                                            }}
                                        >
                                            <Plus className="h-4 w-4 mr-2" />
                                            Add Key
                                        </Button>
                                    )}
                                </div>
                            </div>
                        </AccordionContent>
                    </AccordionItem>
                ))}
            </Accordion>

            <AddKeyModal provider={selectedProvider} open={isModalOpen} onOpenChange={setIsModalOpen} />
        </>
    )
}

