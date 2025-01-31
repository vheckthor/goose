import React, { useEffect, useState } from 'react';
import { Check, Plus, Settings, X, Rocket, RefreshCw } from 'lucide-react';
import { Button } from '../../ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/Tooltip';
import { Portal } from '@radix-ui/react-portal';
import { required_keys } from '../models/hardcoded_stuff';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { getActiveProviders } from '../api_keys/utils';
import { checkOllama } from './utils';

// Types
interface Provider {
  id: string;
  name: string;
  isConfigured: boolean;
  description: string;
}

interface BaseProviderCardProps {
  name: string;
  description: string;
  isConfigured: boolean;
  isSelected?: boolean;
  isSelectable?: boolean;
  hasRequiredKeys?: boolean;
  showSettings?: boolean;
  showDelete?: boolean;
  showTakeoff?: boolean;
  onSelect?: () => void;
  onAddKeys?: () => void;
  onConfigure?: () => void;
  onDelete?: () => void;
  onTakeoff?: () => void;
}

interface BaseProviderGridProps {
  providers: Provider[];
  isSelectable?: boolean;
  showSettings?: boolean;
  showDelete?: boolean;
  selectedId?: string | null;
  onSelect?: (providerId: string) => void;
  onAddKeys?: (provider: Provider) => void;
  onConfigure?: (provider: Provider) => void;
  onDelete?: (provider: Provider) => void;
  onTakeoff?: (provider: Provider) => void;
  showTakeoff?: boolean;
}

// Utilities
const getArticle = (word: string): string =>
    'aeiouAEIOU'.indexOf(word[0]) >= 0 ? 'an' : 'a';

const providerDescriptions: Record<string, string> = {
  OpenAI: 'Access GPT-4, GPT-3.5 Turbo, and other OpenAI models',
  Anthropic: 'Access Claude and other Anthropic models',
  Google: 'Access Gemini and other Google AI models',
  Groq: 'Access Mixtral and other Groq-hosted models',
  Databricks: 'Access models hosted on your Databricks instance',
  OpenRouter: 'Access a variety of AI models through OpenRouter',
  Ollama: 'Run and use open-source models locally',
};

export const getProviderDescription = (provider: string): string =>
    providerDescriptions[provider] || `Access ${provider} models`;

const getGreenCheckTooltipMessage = async (name: string): Promise<string> => {
  if (name === 'Ollama') {
    const ollamaConfig = await checkOllama();
    // If not configured at all, return early
    if (!ollamaConfig.is_set) {
      return '';
    }
    return ollamaConfig.location === 'app'
        ? 'Ollama is running locally'
        : 'Ollama is configured via OLLAMA_HOST';
  }
  return `You have ${getArticle(name)} ${name} API Key set in your environment`;
};

// Reusable IconTooltipButton component
const IconTooltipButton: React.FC<{
  icon: React.ReactNode;
  tooltipText: React.ReactNode;
  onClick: (e: React.MouseEvent) => void;
  className?: string;
  variant?: "ghost" | "default";
}> = ({ icon, tooltipText, onClick, className = '', variant = 'default' }) => (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
              variant={variant}
              size="sm"
              onClick={onClick}
              className={`rounded-full h-7 w-7 p-0 bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-borderStandard hover:text-textStandard transition-colors ${className}`}
          >
            {icon}
          </Button>
        </TooltipTrigger>
        <Portal>
          <TooltipContent side="top" align="center" className="z-[9999]">
            <p>{tooltipText}</p>
          </TooltipContent>
        </Portal>
      </Tooltip>
    </TooltipProvider>
);

// Status indicator component
const StatusIndicator: React.FC<{
  isConfigured: boolean;
  name: string;
  tooltipMessage: React.ReactNode;
  localConfigured?: boolean;
}> = ({ isConfigured, name, tooltipMessage, localConfigured }) => {
  // For Ollama, use localConfigured state if available
  const effectiveConfigured = name === 'Ollama' ? (localConfigured ?? isConfigured) : isConfigured;

  if (effectiveConfigured) {
    return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="flex items-center justify-center w-5 h-5 rounded-full bg-green-100 dark:bg-green-900/30 shrink-0">
                <Check className="h-3 w-3 text-green-600 dark:text-green-500" />
              </div>
            </TooltipTrigger>
            <Portal>
              <TooltipContent side="top" align="center" className="z-[9999]">
                <p>{tooltipMessage}</p>
              </TooltipContent>
            </Portal>
          </Tooltip>
        </TooltipProvider>
    );
  }

  if (name === 'Ollama') {
    return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="flex items-center justify-center w-5 h-5 rounded-full bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-borderStandard hover:text-textStandard transition-colors">
                !
              </div>
            </TooltipTrigger>
            <Portal>
              <TooltipContent side="top" align="center" className="z-[9999]">
                <p>{tooltipMessage}</p>
              </TooltipContent>
            </Portal>
          </Tooltip>
        </TooltipProvider>
    );
  }

  return null;
};

// Card Actions component
const CardActions: React.FC<{
  refreshing?: boolean;
  name: string;
  isConfigured: boolean;
  localConfigured?: boolean;
  hasRequiredKeys: boolean;
  showSettings?: boolean;
  showDelete?: boolean;
  showTakeoff?: boolean;
  onAddKeys?: () => void;
  onConfigure?: () => void;
  onDelete?: () => void;
  onTakeoff?: () => void;
  onRefresh?: () => Promise<void>;
}> = ({
        name,
        isConfigured,
        localConfigured,
        hasRequiredKeys,
        showSettings,
        showDelete,
        showTakeoff,
        onAddKeys,
        onConfigure,
        onDelete,
        onTakeoff,
        refreshing = false,
        onRefresh,
      }) => {
  // For Ollama, use localConfigured if available, otherwise use isConfigured
  const effectiveConfigured = name === 'Ollama' ? (localConfigured ?? isConfigured) : isConfigured;

  return (
      <div className="space-x-2 text-center flex items-center justify-between">
        <div className="space-x-2">
          {!effectiveConfigured && name === 'Ollama' && (
              <IconTooltipButton
                  icon={<RefreshCw className={`!size-4 ${refreshing ? 'animate-spin' : ''}`} />}
                  tooltipText="Click to re-check Ollama configuration status"
                  onClick={async (e) => {
                    e.stopPropagation();
                    onRefresh?.();
                  }}
              />
          )}

          {!effectiveConfigured && onAddKeys && hasRequiredKeys && (
              <IconTooltipButton
                  icon={<Plus className="!size-4" />}
                  tooltipText={`Add ${name} API Key(s)`}
                  onClick={(e) => {
                    e.stopPropagation();
                    onAddKeys();
                  }}
              />
          )}

          {effectiveConfigured && showSettings && hasRequiredKeys && (
              <IconTooltipButton
                  icon={<Settings className="!size-4" />}
                  tooltipText={`Configure ${name} settings`}
                  onClick={(e) => {
                    e.stopPropagation();
                    onConfigure?.();
                  }}
                  variant="ghost"
              />
          )}

          {showDelete && hasRequiredKeys && effectiveConfigured && (
              <IconTooltipButton
                  icon={<X className="!size-4" />}
                  tooltipText={`Remove ${name} API Key or Host`}
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete?.();
                  }}
                  variant="ghost"
              />
          )}
        </div>

        {effectiveConfigured && onTakeoff && showTakeoff !== false && (
            <IconTooltipButton
                icon={<Rocket className="!size-4" />}
                tooltipText={`Launch goose with ${name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onTakeoff();
                }}
            />
        )}
      </div>
  );
};

// Main BaseProviderCard component
function BaseProviderCard({
                            name,
                            description,
                            isConfigured,
                            isSelected,
                            isSelectable,
                            hasRequiredKeys = false,
                            showSettings,
                            showDelete = false,
                            showTakeoff,
                            onSelect,
                            onAddKeys,
                            onConfigure,
                            onDelete,
                            onTakeoff,
                          }: BaseProviderCardProps) {
  const [configuredTooltipMessage, setConfiguredTooltipMessage] = useState('');
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [localConfigured, setLocalConfigured] = useState(isConfigured);
  const { setActiveKeys } = useActiveKeys();
  const numRequiredKeys = required_keys[name]?.length || 0;

  const refreshConfiguration = async () => {
    setIsRefreshing(true);
    try {
      // Check Ollama status first
      if (name === 'Ollama') {
        const ollamaStatus = await checkOllama();
        // Update local configuration state based on actual status
        setLocalConfigured(ollamaStatus.is_set);
      }

      // Refresh active providers
      const providers = await getActiveProviders();
      setActiveKeys(providers);

      // Update tooltip message only if still configured
      const message = await getGreenCheckTooltipMessage(name);
      setConfiguredTooltipMessage(message);
    } catch (error) {
      console.error('Error refreshing configuration:', error);
      // On error, assume not configured
      if (name === 'Ollama') {
        setLocalConfigured(false);
        setConfiguredTooltipMessage('');
      }
    } finally {
      setIsRefreshing(false);
    }
  };

  // Effect to handle initial load and configuration changes
  useEffect(() => {
    const fetchMessage = async () => {
      if (name === 'Ollama') {
        const ollamaStatus = await checkOllama();
        setLocalConfigured(ollamaStatus.is_set);
        if (ollamaStatus.is_set) {
          const message = await getGreenCheckTooltipMessage(name);
          setConfiguredTooltipMessage(message);
        } else {
          setConfiguredTooltipMessage('');
        }
      } else {
        const message = await getGreenCheckTooltipMessage(name);
        setConfiguredTooltipMessage(message);
      }
    };

    fetchMessage();

    // If this is the Ollama provider, set up an interval to check status
    let intervalId: NodeJS.Timeout | null = null;
    if (name === 'Ollama') {
      intervalId = setInterval(fetchMessage, 5000); // Check every 5 seconds
    }

    // Cleanup interval on unmount
    return () => {
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [name, isConfigured]);

  const ollamaTooltipContent = (
      <>
        To use, either the{' '}
        <a
            href="https://ollama.com/download"
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-600 underline hover:text-blue-800"
        >
          Ollama app
        </a>{' '}
        must be installed on your machine and open, or you must enter a value for OLLAMA_HOST.
      </>
  );

  return (
      <div className="relative h-full p-[2px] overflow-hidden rounded-[9px] group/card bg-borderSubtle hover:bg-transparent hover:duration-300">
        <div
            className={`absolute pointer-events-none w-[260px] h-[260px] top-[-50px] left-[-30px] origin-center bg-[linear-gradient(45deg,#13BBAF,#FF4F00)] animate-[rotate_6s_linear_infinite] z-[-1] ${
                isSelected ? 'opacity-100' : 'opacity-0 group-hover/card:opacity-100'
            }`}
        />
        <div
            onClick={() => isSelectable && isConfigured && onSelect?.()}
            className={`relative bg-bgApp rounded-lg p-3 transition-all duration-200 h-[160px] flex flex-col justify-between
          ${isSelectable && isConfigured ? 'cursor-pointer' : ''}
          ${!isSelectable || (isSelectable && isConfigured) ? 'hover:border-borderStandard' : ''}`}
        >
          <div>
            <div className="flex items-center">
              <h3 className="text-base font-medium text-textStandard truncate mr-2">{name}</h3>
              <StatusIndicator
                  isConfigured={isConfigured}
                  localConfigured={localConfigured}
                  name={name}
                  tooltipMessage={isConfigured ? configuredTooltipMessage : ollamaTooltipContent}
              />
            </div>
            <p className="text-xs text-textSubtle mt-1.5 mb-3 leading-normal overflow-y-auto max-h-[54px]">
              {description}
            </p>
          </div>

          <CardActions
              name={name}
              isConfigured={isConfigured}
              localConfigured={localConfigured}
              hasRequiredKeys={hasRequiredKeys}
              showSettings={showSettings}
              showDelete={showDelete}
              showTakeoff={showTakeoff}
              onAddKeys={onAddKeys}
              onConfigure={onConfigure}
              onDelete={onDelete}
              onTakeoff={onTakeoff}
              refreshing={isRefreshing}
              onRefresh={refreshConfiguration}
          />
        </div>
      </div>
  );
}

// Grid component
export function BaseProviderGrid({
                                   providers,
                                   isSelectable = false,
                                   showSettings = false,
                                   showDelete = false,
                                   selectedId = null,
                                   onSelect,
                                   onAddKeys,
                                   onConfigure,
                                   onDelete,
                                   showTakeoff,
                                   onTakeoff,
                                 }: BaseProviderGridProps) {
  return (
      <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-7 gap-3 auto-rows-fr max-w-full [&_*]:z-20">
        {providers.map((provider) => {
          const hasRequiredKeys = required_keys[provider.name]?.length > 0;
          return (
              <BaseProviderCard
                  key={provider.id}
                  name={provider.name}
                  description={provider.description}
                  isConfigured={provider.isConfigured}
                  isSelected={selectedId === provider.id}
                  isSelectable={isSelectable}
                  onSelect={() => onSelect?.(provider.id)}
                  onAddKeys={() => onAddKeys?.(provider)}
                  onConfigure={() => onConfigure?.(provider)}
                  onDelete={() => onDelete?.(provider)}
                  onTakeoff={() => onTakeoff?.(provider)}
                  showSettings={showSettings}
                  showDelete={showDelete}
                  hasRequiredKeys={hasRequiredKeys}
                  showTakeoff={showTakeoff}
              />
          );
        })}
      </div>
  );
}