import React, { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { Buffer } from 'buffer';
import { FullExtensionConfig } from '../extensions';
import { ChevronRight } from './icons/ChevronRight';
import Back from './icons/Back';
import { Bars } from './icons/Bars';
import { Geese } from './icons/Geese';
import Copy from './icons/Copy';
<<<<<<< HEAD
import Check from './icons/Check';
import { useConfig } from '../components/ConfigContext';
||||||| parent of 2020cd26f (fix: update extensions page on recipe)
import { useConfig } from '../components/ConfigContext';
=======
import { useConfig } from './ConfigContext';
>>>>>>> 2020cd26f (fix: update extensions page on recipe)
import { settingsV2Enabled } from '../flags';
import ExtensionsSection from './settings_v2/extensions/ExtensionsSection';
import { FixedExtensionEntry } from './ConfigContext';

interface RecipeEditorProps {
  config?: Recipe;
}

// Function to generate a deep link from a recipe
function generateDeepLink(recipe: Recipe): string {
  const configBase64 = Buffer.from(JSON.stringify(recipe)).toString('base64');
  return `goose://recipe?config=${configBase64}`;
}

export default function RecipeEditor({ config }: RecipeEditorProps) {
  const { getExtensions } = useConfig();
  const [recipeConfig] = useState<Recipe | undefined>(config);
  const [title, setTitle] = useState(config?.title || '');
  const [description, setDescription] = useState(config?.description || '');
  const [instructions, setInstructions] = useState(config?.instructions || '');
  const [activities, setActivities] = useState<string[]>(config?.activities || []);
  const [availableExtensions, setAvailableExtensions] = useState<FullExtensionConfig[]>([]);
  const [selectedExtensions, setSelectedExtensions] = useState<string[]>(
    config?.extensions?.map((e) => e.id) || []
  );
  const [newActivity, setNewActivity] = useState('');
  const [copied, setCopied] = useState(false);

  // Section visibility state
  const [activeSection, setActiveSection] = useState<
    'none' | 'activities' | 'instructions' | 'extensions'
  >('none');

  // Load extensions
  useEffect(() => {
    const loadExtensions = async () => {
      if (settingsV2Enabled) {
        try {
          const extensions = await getExtensions(false); // force refresh to get latest
          console.log('extensions {}', extensions);
          // Set all extensions to disabled by default in recipe editor
          const disabledExtensions =
            extensions?.map((ext) => ({
              ...ext,
              enabled: false,
            })) || [];
          setAvailableExtensions(disabledExtensions);
        } catch (error) {
          console.error('Failed to load extensions:', error);
        }
      } else {
        const userSettingsStr = localStorage.getItem('user_settings');
        if (userSettingsStr) {
          const userSettings = JSON.parse(userSettingsStr);
          const disabledExtensions = (userSettings.extensions || []).map((ext) => ({
            ...ext,
            enabled: false,
          }));
          setAvailableExtensions(disabledExtensions);
        }
      }
    };
    loadExtensions();
    // Intentionally omitting getExtensions from deps to avoid refresh loops
    // eslint-disable-next-line
  }, []);

  const handleExtensionToggle = (extension: FixedExtensionEntry) => {
    console.log('Toggling extension:', extension.name);
    setSelectedExtensions((prev) => {
      const isSelected = prev.includes(extension.name);
      const newState = isSelected
        ? prev.filter((extId) => extId !== extension.name)
        : [...prev, extension.name];
      return newState;
    });
  };

  const handleAddActivity = () => {
    if (newActivity.trim()) {
      setActivities((prev) => [...prev, newActivity.trim()]);
      setNewActivity('');
    }
  };

  const handleRemoveActivity = (activity: string) => {
    setActivities((prev) => prev.filter((a) => a !== activity));
  };

  const getCurrentConfig = (): Recipe => {
    console.log('Creating config with:', {
      selectedExtensions,
      availableExtensions,
      recipeConfig,
    });

    const config = {
      ...recipeConfig,
      title,
      description,
      instructions,
      activities,
      extensions: selectedExtensions
        .map((name) => {
          const extension = availableExtensions.find((e) => e.name === name);
          console.log('Looking for extension:', name, 'Found:', extension);
          if (!extension) return null;

          // Create a clean copy of the extension configuration
          const cleanExtension = { ...extension };
          delete cleanExtension.enabled;

          // If the extension has env_keys, preserve keys but clear values
          if (cleanExtension.env_keys) {
            cleanExtension.env_keys = Object.fromEntries(
              Object.keys(cleanExtension.env_keys).map((key) => [key, ''])
            );
          }

          return cleanExtension;
        })
        .filter(Boolean) as FullExtensionConfig[],
    };
    console.log('Final config extensions:', config.extensions);
    return config;
  };

  const [errors, setErrors] = useState<{ title?: string; description?: string }>({});

  const validateForm = () => {
    const newErrors: { title?: string; description?: string } = {};
    if (!title.trim()) {
      newErrors.title = 'Title is required';
    }
    if (!description.trim()) {
      newErrors.description = 'Description is required';
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleOpenAgent = () => {
    if (validateForm()) {
      const updatedConfig = getCurrentConfig();
      window.electron.createChatWindow(
        undefined,
        undefined,
        undefined,
        undefined,
        updatedConfig,
        undefined
      );
    }
  };

  const deeplink = generateDeepLink(getCurrentConfig());

  const handleCopy = () => {
    // Copy the text to the clipboard
    navigator.clipboard
      .writeText(deeplink)
      .then(() => {
        setCopied(true); // Show the check mark
        // Reset to normal after 2 seconds (2000 milliseconds)
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((err) => {
        console.error('Failed to copy the text:', err);
      });
  };

  // Render expanded section content
  const renderSectionContent = () => {
    switch (activeSection) {
      case 'activities':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6 text-iconProminent" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-iconSubtle" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2 text-textProminent">Activities</h2>
              <p className="text-textSubtle">
                The top-line prompts and activities that will display within your goose home page.
              </p>
            </div>
            <div className="space-y-4">
              <div className="flex flex-wrap gap-3">
                {activities.map((activity, index) => (
                  <div
                    key={index}
                    className="inline-flex items-center bg-bgApp border-2 border-borderSubtle rounded-full px-4 py-2 text-sm text-textStandard"
                    title={activity.length > 100 ? activity : undefined}
                  >
                    <span>{activity.length > 100 ? activity.slice(0, 100) + '...' : activity}</span>
                    <button
                      onClick={() => handleRemoveActivity(activity)}
                      className="ml-2 text-textStandard hover:text-textSubtle transition-colors"
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
              <div className="flex gap-3 mt-6">
                <input
                  type="text"
                  value={newActivity}
                  onChange={(e) => setNewActivity(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleAddActivity()}
                  className="flex-1 px-4 py-3 bg-bgSubtle text-textStandard rounded-xl placeholder-textPlaceholder focus:outline-none focus:ring-2 focus:ring-borderProminent"
                  placeholder="Add new activity..."
                />
                <button
                  onClick={handleAddActivity}
                  className="px-5 py-3 bg-bgAppInverse text-textProminentInverse rounded-xl hover:bg-bgStandardInverse transition-colors"
                >
                  Add activity
                </button>
              </div>
            </div>
          </div>
        );

      case 'instructions':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6 text-iconProminent" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-iconSubtle" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2 text-textProminent">Instructions</h2>
              <p className="text-textSubtle">
                Hidden instructions that will be passed to the provider to help direct and add
                context to your responses.
              </p>
            </div>
            <textarea
              value={instructions}
              onChange={(e) => setInstructions(e.target.value)}
              className="w-full h-96 p-4 bg-bgSubtle text-textStandard rounded-xl resize-none focus:outline-none focus:ring-2 focus:ring-borderProminent"
              placeholder="Enter instructions..."
            />
          </div>
        );

      case 'extensions':
        return (
          <div className="p-6 pt-10">
            <button onClick={() => setActiveSection('none')} className="mb-6">
              <Back className="w-6 h-6 text-iconProminent" />
            </button>
            <div className="py-2">
              <Bars className="w-6 h-6 text-iconSubtle" />
            </div>
            <div className="mb-8 mt-6">
              <h2 className="text-2xl font-medium mb-2 text-textProminent">Extensions</h2>
            </div>
            <ExtensionsSection
              hideButtons={true} // Hide "Add custom" and "Browse" buttons
              hideHeader={true} // Hide the section header since we have our own
              disableConfiguration={true} // Hide configuration
              customToggle={async (extension: FixedExtensionEntry) => {
                handleExtensionToggle(extension);
              }}
            />
          </div>
        );

      default:
        return (
          <div className="space-y-4 py-4">
            <div>
              <h2 className="text-lg font-medium mb-2 text-textProminent">Agent</h2>
              <input
                type="text"
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value);
                  if (errors.title) {
                    setErrors({ ...errors, title: undefined });
                  }
                }}
                className={`w-full p-3 border rounded-lg bg-bgApp text-textStandard ${
                  errors.title ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Agent Name (required)"
              />
              {errors.title && <div className="text-red-500 text-sm mt-1">{errors.title}</div>}
            </div>

            <div>
              <input
                type="text"
                value={description}
                onChange={(e) => {
                  setDescription(e.target.value);
                  if (errors.description) {
                    setErrors({ ...errors, description: undefined });
                  }
                }}
                className={`w-full p-3 border rounded-lg bg-bgApp text-textStandard ${
                  errors.description ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Description (required)"
              />
              {errors.description && (
                <div className="text-red-500 text-sm mt-1">{errors.description}</div>
              )}
            </div>

            {/* Section buttons */}
            <button
              onClick={() => setActiveSection('activities')}
              className="w-full flex items-start justify-between p-4 border border-borderSubtle rounded-lg bg-bgApp hover:bg-bgSubtle"
            >
              <div className="text-left">
                <h3 className="font-medium text-textProminent">Activities</h3>
                <p className="text-textSubtle text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1 text-iconSubtle" />
            </button>

            <button
              onClick={() => setActiveSection('instructions')}
              className="w-full flex items-start justify-between p-4 border border-borderSubtle rounded-lg bg-bgApp hover:bg-bgSubtle"
            >
              <div className="text-left">
                <h3 className="font-medium text-textProminent">Instructions</h3>
                <p className="text-textSubtle text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1 text-iconSubtle" />
            </button>

            <button
              onClick={() => setActiveSection('extensions')}
              className="w-full flex items-start justify-between p-4 border border-borderSubtle rounded-lg bg-bgApp hover:bg-bgSubtle"
            >
              <div className="text-left">
                <h3 className="font-medium text-textProminent">Extensions</h3>
                <p className="text-textSubtle text-sm">
                  Starting activities present in the home panel on a fresh goose session
                </p>
              </div>
              <ChevronRight className="w-5 h-5 mt-1 text-iconSubtle" />
            </button>

            {/* Deep Link Display */}
            <div className="w-full p-4 bg-bgSubtle rounded-lg flex items-center justify-between">
              <code className="text-sm text-textSubtle truncate">{deeplink}</code>
              <button
                onClick={handleCopy}
                className="ml-2 disabled:opacity-50 disabled:cursor-not-allowed"
                disabled={!title.trim() || !description.trim()}
              >
                {copied ? (
                  <Check className="w-5 h-5 text-green-500" />
                ) : (
                  <Copy className="w-5 h-5 text-iconSubtle" />
                )}
              </button>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-col space-y-2 pt-4">
              <button
                onClick={handleOpenAgent}
                className="w-full p-3 bg-bgAppInverse text-textProminentInverse rounded-lg hover:bg-bgStandardInverse disabled:opacity-50 disabled:cursor-not-allowed"
                disabled={!title.trim() || !description.trim()}
              >
                Open agent
              </button>
              <button
                onClick={() => window.close()}
                className="w-full p-3 text-textSubtle rounded-lg hover:bg-bgSubtle"
              >
                Cancel
              </button>
            </div>
          </div>
        );
    }
  };

  return (
    <div className="flex flex-col w-full h-screen bg-bgApp max-w-3xl mx-auto">
      {activeSection === 'none' && (
        <div className="flex flex-col items-center mb-6 px-6 pt-10">
          <div className="w-16 h-16 bg-bgApp rounded-full flex items-center justify-center mb-4">
            <Geese className="w-12 h-12 text-iconProminent" />
          </div>
          <h1 className="text-2xl font-medium text-center text-textProminent">
            Create custom agent
          </h1>
          <p className="text-textSubtle text-center mt-2 text-sm">
            Your custom agent can be shared with others
          </p>
        </div>
      )}
      <div className="flex-1 overflow-y-auto px-6">{renderSectionContent()}</div>
    </div>
  );
}
