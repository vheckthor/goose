import React, { useState } from 'react';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import Select from 'react-select';
import { Plus } from 'lucide-react';
import { Switch } from '../../ui/switch';
import { createSelectedModel, useHandleModelSelection } from './utils';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { goose_models, model_docs_link } from './hardcoded_stuff';
import { createDarkSelectStyles, darkSelectTheme } from '../../ui/select-styles';
import { useModel } from './ModelContext';

// Create a mapping from provider name to href
const providerLinks = model_docs_link.reduce((acc, { name, href }) => {
  acc[name] = href;
  return acc;
}, {});

export function ChangeModelInline() {
  const { activeKeys } = useActiveKeys();
  const { currentModel } = useModel();
  const handleModelSelection = useHandleModelSelection();

  const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
  const [modelName, setModelName] = useState<string>('');
  const [isAddingCustom, setIsAddingCustom] = useState(false);

  // Convert active keys to dropdown options
  const providerOptions = activeKeys.map((key) => ({
    value: key.toLowerCase(),
    label: key,
  }));

  // Filter models by selected provider
  const providerModels = selectedProvider
    ? goose_models.filter(
        (model) => model.provider.toLowerCase() === selectedProvider.toLowerCase()
      )
    : [];

  const handleAddCustomModel = () => {
    if (!selectedProvider || !modelName) {
      console.error('Both provider and model name are required.');
      return;
    }

    const selectedModel = createSelectedModel(selectedProvider, modelName);
    handleModelSelection(selectedModel, 'ChangeModelInline');

    // Reset form state
    setModelName('');
    setIsAddingCustom(false);
  };

  return (
    <div className="space-y-4">
      <Select
        options={providerOptions}
        value={providerOptions.find((option) => option.value === selectedProvider) || null}
        onChange={(option: any) => {
          setSelectedProvider(option?.value || null);
          setModelName('');
          setIsAddingCustom(false);
        }}
        placeholder="Select provider"
        isClearable
        styles={createDarkSelectStyles('100%')}
        theme={darkSelectTheme}
      />

      {selectedProvider && (
        <div className="space-y-4 mt-2">
          {/* Models List */}
          <div className="border rounded-lg dark:border-gray-700">
            {providerModels.map((model) => (
              <div
                key={model.id}
                className="py-2 px-3 border-b last:border-b-0 dark:border-gray-700
                          flex justify-between items-center hover:bg-gray-50 dark:hover:bg-gray-800"
              >
                <span className="text-sm text-gray-700 dark:text-gray-300">{model.name}</span>
                <Switch
                  variant="mono"
                  checked={model.id === currentModel?.id}
                  onCheckedChange={() => handleModelSelection(model, 'ChangeModelInline')}
                />
              </div>
            ))}
          </div>

          {/* Links and Custom Model Input */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <a
                href={providerLinks[selectedProvider]}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
              >
                Browse more {selectedProvider} models...
              </a>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setIsAddingCustom(!isAddingCustom)}
                className="text-sm text-gray-600 dark:text-gray-400"
              >
                {isAddingCustom ? 'Cancel' : 'Add Custom Model'}
              </Button>
            </div>

            {isAddingCustom && (
              <div className="space-y-2">
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Don't see your model? Enter its name below to add it:
                </p>
                <div className="flex gap-2">
                  <Input
                    type="text"
                    placeholder="Enter model name"
                    value={modelName}
                    onChange={(e) => setModelName(e.target.value)}
                    className="flex-1"
                  />
                  <Button
                    type="button"
                    onClick={handleAddCustomModel}
                    className="bg-black text-white hover:bg-black/90"
                  >
                    <Plus className="mr-2 h-4 w-4" /> Add
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
