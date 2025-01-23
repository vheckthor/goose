import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import {
  Modal,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
} from '../ui/modal';
import { Input } from '../ui/input';
import { Label } from '../ui/label';

type Provider = {
  id: string;
  name: string;
};

const providers: Provider[] = [
  { id: 'openai', name: 'OpenAI' },
  { id: 'anthropic', name: 'Anthropic' },
  { id: 'google', name: 'Google' },
  { id: 'ollama', name: 'Ollama' },
  { id: 'groq', name: 'Groq' },
  { id: 'openrouter', name: 'OpenRouter' },
  { id: 'databricks', name: 'Databricks' },
];

export function WelcomeModal({
  onSubmit,
}: {
  onSubmit: (apiKey: string, providerId: Provider) => void;
}) {
  const [selectedProvider, setSelectedProvider] = useState<Provider | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleProviderSelect = (provider: Provider) => {
    console.log('Selected Provider:', provider); // Debugging line
    setSelectedProvider(provider);
    setIsModalOpen(true);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedProvider) {
      console.error('No provider selected'); // Debugging line
      return;
    }
    onSubmit(apiKey, selectedProvider); // Assuming onSubmit is responsible for handling the selected provider and key
    setApiKey('');
    setIsModalOpen(false);
  };

  return (
    <div className="h-full w-full bg-white dark:bg-gray-800 p-8 space-y-6">
      <h1 className="text-xl font-semibold text-gray-800 dark:text-white">Welcome to Goose</h1>
      <p className="text-sm text-gray-600 dark:text-gray-300 mb-4">
        Select a provider to get started:
      </p>
      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-4">
        {providers.map((provider) => (
          <Card key={provider.id} className="hover:shadow-lg transition-shadow">
            <CardHeader className="p-3">
              <CardTitle className="text-base">{provider.name}</CardTitle>
            </CardHeader>
            <CardContent className="p-3">
              <Button
                onClick={() => handleProviderSelect(provider)}
                className="w-full text-sm bg-indigo-500 hover:bg-indigo-600 text-white"
                size="sm"
              >
                Configure
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>

      {isModalOpen && (
        <Modal open={isModalOpen} onOpenChange={setIsModalOpen}>
          <ModalContent className="sm:max-w-[425px]">
            <ModalHeader>
              <ModalTitle className="text-lg font-semibold">
                Configure {selectedProvider?.name}
              </ModalTitle>
              <ModalDescription className="text-sm text-gray-600 dark:text-gray-300">
                Enter your API key for {selectedProvider?.name} to get started.
              </ModalDescription>
            </ModalHeader>
            <form onSubmit={handleSubmit}>
              <div className="grid gap-4 py-4">
                <div className="grid grid-cols-4 items-center gap-4">
                  <Label htmlFor="apiKey">API Key</Label>
                  <Input
                    id="apiKey"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    className="col-span-3"
                    required
                  />
                </div>
              </div>
              <ModalFooter>
                <Button type="submit" className="bg-indigo-500 hover:bg-indigo-600 text-white">
                  Submit
                </Button>
              </ModalFooter>
            </form>
          </ModalContent>
        </Modal>
      )}
    </div>
  );
}
