import React, { useEffect, useState } from 'react';
import { Config } from '../../api/config';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Card } from '../ui/card';
import { useNavigate } from 'react-router-dom';
import BackButton from '../ui/BackButton';
import type { View } from '@/src/ChatWindow';

interface ConfigItem {
  key: string;
  value: any;
}

export function ConfigPage({
  onClose,
  setView,
}: {
  onClose: () => void;
  setView?: (view: View) => void;
}) {
  const [configs, setConfigs] = useState<Record<string, any>>({});
  const [loading, setLoading] = useState(true);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  // Fetch all configs on component mount
  useEffect(() => {
    loadConfigs();
  }, []);

  const loadConfigs = async () => {
    try {
      setLoading(true);
      console.log('[loadConfigs] attempting to load all configs');
      const response = await Config.readAll();
      console.log('[loadConfigs] result / response:', response);
      setConfigs(response || {});
      setError(null);
    } catch (err) {
      setError('Failed to load configurations');
      console.error('Error loading configs:', err);
      setConfigs({});
    } finally {
      setLoading(false);
    }
  };

  const handleAddConfig = async () => {
    if (!newKey || !newValue) {
      setError('Both key and value are required');
      return;
    }

    try {
      let parsedValue = newValue;
      // Try to parse as JSON if it looks like JSON
      if (newValue.trim().startsWith('{') || newValue.trim().startsWith('[')) {
        try {
          parsedValue = JSON.parse(newValue);
        } catch (e) {
          // If parsing fails, use the original string value
          console.log('Value is not valid JSON, using as string');
        }
      }
      console.log('[handleAddConfig] upserting key:', newKey, 'value', parsedValue);
      await Config.upsert(newKey, parsedValue);
      console.log('[handleAddConfig] success upserting key');
      setNewKey('');
      setNewValue('');
      setError(null);
      console.log('[handleAddConfig] reloading the configs');
      await loadConfigs(); // Reload the configs
      console.log('[handleAddConfig] success reloading');
    } catch (err) {
      setError('Failed to add configuration');
      console.error('Error adding config:', err);
    }
  };

  const handleRemoveConfig = async (key: string) => {
    try {
      await Config.remove(key);
      await loadConfigs(); // Reload the configs
      setError(null);
    } catch (err) {
      setError(`Failed to remove configuration: ${key}`);
      console.error('Error removing config:', err);
    }
  };

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <div className="flex flex-col pb-24">
        <div className="px-8 pt-6 pb-4">
          <BackButton onClick={() => navigate('/settings')} />
          <h1 className="text-3xl font-medium text-textStandard mt-1">Configuration</h1>
        </div>

        <div className="flex-1 py-8 pt-[20px] px-8">
          <div className="space-y-8 max-w-2xl">
            {/* Add new config form */}
            <Card className="p-6">
              <h2 className="text-xl font-semibold mb-4">Add New Configuration</h2>
              <div className="grid gap-4">
                <div>
                  <Label htmlFor="configKey">Key</Label>
                  <Input
                    id="configKey"
                    value={newKey}
                    onChange={(e) => setNewKey(e.target.value)}
                    placeholder="Enter config key"
                    className="mt-1"
                  />
                </div>
                <div>
                  <Label htmlFor="configValue">Value</Label>
                  <Input
                    id="configValue"
                    value={newValue}
                    onChange={(e) => setNewValue(e.target.value)}
                    placeholder="Enter config value (string or JSON)"
                    className="mt-1"
                  />
                </div>
                <Button onClick={handleAddConfig}>Add Configuration</Button>
              </div>
            </Card>

            {/* Error display */}
            {error && (
              <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
                {error}
              </div>
            )}

            {/* Config list */}
            <Card className="p-6">
              <h2 className="text-xl font-semibold mb-4">Current Configurations</h2>
              <div className="grid gap-4">
                {loading ? (
                  <div className="text-center text-gray-500">Loading configurations...</div>
                ) : Object.keys(configs).length === 0 ? (
                  <div className="text-center text-gray-500">No configurations found</div>
                ) : (
                  Object.entries(configs).map(([key, value]) => (
                    <div
                      key={key}
                      className="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-800 rounded"
                    >
                      <div className="break-all">
                        <span className="font-medium">{key}:</span>{' '}
                        <span className="text-gray-600 dark:text-gray-300">
                          {typeof value === 'object'
                            ? JSON.stringify(value, null, 2)
                            : String(value)}
                        </span>
                      </div>
                      <Button
                        variant="destructive"
                        onClick={() => handleRemoveConfig(key)}
                        size="sm"
                        className="ml-4 shrink-0"
                      >
                        Remove
                      </Button>
                    </div>
                  ))
                )}
              </div>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
