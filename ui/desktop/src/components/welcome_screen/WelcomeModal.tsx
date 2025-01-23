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
  onSubmit: (apiKey: string, providerId: string) => void;
}) {
  const [selectedProvider, setSelectedProvider] = useState<Provider | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleProviderSelect = (provider: Provider) => {
    console.log('Selected Provider:', provider);
    setSelectedProvider(provider);
    setIsModalOpen(true);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (selectedProvider) {
      onSubmit(apiKey, selectedProvider.id); // Call the parent's onSubmit with the API key and provider ID
    }
    setApiKey(''); // Reset API key field
    setIsModalOpen(false); // Close the modal
  };

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-2xl font-bold mb-6">Welcome to Goose</h1>
      <p className="mb-4">Select a provider to get started:</p>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {providers.map((provider) => (
          <Card key={provider.id} className="cursor-pointer hover:shadow-md transition-shadow">
            <CardHeader className="flex flex-row items-center space-x-4">
              <CardTitle>{provider.name}</CardTitle>
            </CardHeader>
            <CardContent>
              <Button onClick={() => handleProviderSelect(provider)} className="w-full">
                Configure
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>

      <Modal open={isModalOpen} onOpenChange={setIsModalOpen}>
        <ModalContent>
          <ModalHeader>
            <ModalTitle>Configure {selectedProvider?.name}</ModalTitle>
            <ModalDescription>
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
              <Button type="submit">Submit</Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>
    </div>
  );
}
