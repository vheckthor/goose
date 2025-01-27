import {useState} from "react";
import {useNavigate} from "react-router-dom";
import {FullExtensionConfig, BUILT_IN_EXTENSIONS} from "../../extensions";
import {ScrollArea} from "@radix-ui/themes";
import BackButton from "../ui/BackButton";
import {ModelsSection} from "./models/ModelsSection";
import React from "react";
import {ExtensionsSection} from "./extensions/ExtensionsSection";
import {ConfigureExtensionModal} from "./extensions/ConfigureExtensionModal";
import {ManualExtensionModal} from "./extensions/ManualExtensionModal";
import {toast} from "react-toastify";
import {ConfigureBuiltInExtensionModal} from "./extensions/ConfigureBuiltInExtensionModal";
import { Settings as SettingsType } from './types';
import { useStoredExtensions } from './extensions/StoredExtensionsContext';

const DEFAULT_SETTINGS: SettingsType = {
  // @ts-expect-error "we actually do always have all the properties required for builtins, but tsc cannot tell for some reason"
  extensions: BUILT_IN_EXTENSIONS,
};

export default function Settings() {
  const navigate = useNavigate();
  // Access extensions and toggleExtension from context
  const { storedExtensions: extensions, toggleExtension, addExtension, removeExtension } = useStoredExtensions();

  const [extensionBeingConfigured, setExtensionBeingConfigured] =
      useState<FullExtensionConfig | null>(null);
  const [isManualModalOpen, setIsManualModalOpen] = useState(false);

  const handleManualExtensionSubmit = async (extension: FullExtensionConfig) => {
    await addExtension(extension);
    setIsManualModalOpen(false);
  };

  const handleExtensionConfigSubmit = () => {
    setExtensionBeingConfigured(null);
  };

  const isBuiltIn = (extensionId: string) => {
    return BUILT_IN_EXTENSIONS.some((builtIn) => builtIn.id === extensionId);
  };

  const handleExtensionRemove = async (extension: FullExtensionConfig) => {
    if (!extensionBeingConfigured) return;

    try {
      // Remove extension from localStorage and context
      removeExtension(extension.id)
      // Notify the user
      toast.success(`Successfully removed ${extensionBeingConfigured.name} extension`);

      // Close the modal and reset the state
      setExtensionBeingConfigured(null);
    } catch (error) {
      console.error('Failed to remove extension:', error);
      toast.error(`Failed to remove ${extensionBeingConfigured.name} extension`);
    }
  };

  return (
      <div className="h-screen w-full">
        <ScrollArea className="h-full w-full">
          <div className="flex flex-col pb-24">
            <div className="px-8 pt-6 pb-4">
              <BackButton onClick={() => navigate('/chat/1', { replace: true })} />
              <h1 className="text-3xl font-medium text-textStandard mt-1">Settings</h1>
            </div>

            <div className="flex-1 py-8 pt-[20px] space-y-8">
              {/* Models Section */}
              <ModelsSection onBrowse={() => navigate('/settings/more-models')} />

              {/* Extensions Section */}
              <ExtensionsSection
                  extensions={extensions}  // Pass extensions from context
                  onToggle={toggleExtension} // Pass extension setting from context
                  onConfigure={setExtensionBeingConfigured}
                  onAddManualExtension={() => setIsManualModalOpen(true)}
              />
            </div>
          </div>
        </ScrollArea>

        {/* Modals */}
        {extensionBeingConfigured && isBuiltIn(extensionBeingConfigured.id) ? (
            <ConfigureBuiltInExtensionModal
                isOpen={!!extensionBeingConfigured && isBuiltIn(extensionBeingConfigured.id)}
                onClose={() => {
                  setExtensionBeingConfigured(null);
                  navigate('/settings', { replace: true });
                }}
                extension={extensionBeingConfigured}
                onSubmit={handleExtensionConfigSubmit}
            />
        ) : (
            <ConfigureExtensionModal
                isOpen={!!extensionBeingConfigured}
                onClose={() => {
                  setExtensionBeingConfigured(null);
                  navigate('/settings', { replace: true });
                }}
                extension={extensionBeingConfigured}
                onSubmit={handleExtensionConfigSubmit}
                onRemove={handleExtensionRemove}
            />
        )}

        {/* Manual Extension Modal */}
        <ManualExtensionModal
            isOpen={isManualModalOpen}
            onClose={() => setIsManualModalOpen(false)}
            onSubmit={handleManualExtensionSubmit}
        />
      </div>
  );
}
