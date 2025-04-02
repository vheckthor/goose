import React, { useEffect, useState } from 'react';
import Model, { getProviderMetadata } from '../modelInterface';
import { useRecentModels } from './recentModels';
import { changeModel, getCurrentModelAndProvider } from '../index';
import { useConfig } from '../../../ConfigContext';
import { getExtensions } from '@/src/api';
import ToastService, { toastService } from '../../../../toasts';

interface ModelRadioListProps {
  renderItem: (props: {
    model: Model;
    isSelected: boolean;
    onSelect: () => void;
  }) => React.ReactNode;
  className?: string;
  providedModelList?: Model[];
}

// renders a model list and handles changing models when user clicks on them
export function BaseModelsList({
  renderItem,
  className = '',
  providedModelList,
}: ModelRadioListProps) {
  const { recentModels } = useRecentModels();

  // allow for a custom model list to be passed if you don't want to use recent models
  let modelList: Model[];
  if (!providedModelList) {
    modelList = recentModels;
  } else {
    modelList = providedModelList;
  }
  const { read, upsert, getExtensions, addExtension } = useConfig();
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Load current model/provider once on component mount
  useEffect(() => {
    let isMounted = true;

    const initializeCurrentModel = async () => {
      try {
        const result = await getCurrentModelAndProvider({ readFromConfig: read });
        if (isMounted) {
          // try to look up the model in the modelList
          let currentModel: Model;
          const match = modelList.find(
            (model) => model.name == result.model && model.provider == result.provider
          );
          // no matches so just create a model object (maybe user updated config.yaml from CLI usage, manual editing etc)
          if (!match) {
            currentModel = { name: result.model, provider: result.provider };
          } else {
            currentModel = match;
          }
          console.log('Checking for set selected model', currentModel);
          setSelectedModel(currentModel);
          setIsInitialized(true);
        }
      } catch (error) {
        console.error('Failed to load current model:', error);
        if (isMounted) {
          setIsInitialized(true); // Still mark as initialized even on error
        }
      }
    };

    initializeCurrentModel().then();

    return () => {
      isMounted = false;
    };
  }, [read]);

  const handleModelSelection = async (model: Model) => {
    // Fix: Use the model parameter that's passed in
    console.log('in handleModelSelection');
    await changeModel({ model: model, writeToConfig: upsert, getExtensions, addExtension });
  };

  // Updated to work with CustomRadio
  const handleRadioChange = async (model: Model) => {
    console.log('In handle Radio Change');
    // Check if the selected model is already active
    if (
      selectedModel &&
      selectedModel.name === model.name &&
      selectedModel.provider === model.provider
    ) {
      console.log(`Model "${model.name}" is already active.`);
      toastService.error({
        title: 'same model already',
        msg: `Model "${model.name}" is already active.`,
        traceback: null,
      });

      return;
    }

    try {
      // Fix: First save the model to config, then update local state
      console.log('about to go into handle model selection');
      await handleModelSelection(model);

      // Update local state after successful save
      console.log('Checking selected model 2', model);
      setSelectedModel(model);
    } catch (error) {
      console.error('Error selecting model:', error);
    }
  };

  // Don't render until we've loaded the initial model/provider
  if (!isInitialized) {
    return <div>Loading models...</div>;
  }

  return (
    <div className={className}>
      {modelList.map((model) => {
        console.log('A string easy to search for. selectedmodel', selectedModel, 'model', model);
        return renderItem({
          model,
          isSelected:
            selectedModel &&
            selectedModel.name === model.name &&
            selectedModel.provider === model.provider,
          onSelect: () => handleRadioChange(model),
        });
      })}
    </div>
  );
}
