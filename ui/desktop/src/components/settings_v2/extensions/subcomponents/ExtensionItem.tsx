import React from 'react';
import { Switch } from '../../../ui/switch';
import { Gear } from '../../../icons/Gear';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { getSubtitle, getFriendlyTitle } from './ExtensionList';

interface ExtensionItemProps {
  extension: FixedExtensionEntry;
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void>;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  disableConfiguration?: boolean;
}

export default function ExtensionItem({
  extension,
  onToggle,
  onConfigure,
  disableConfiguration,
}: ExtensionItemProps) {
  const handleToggle = async (ext: FixedExtensionEntry) => {
    try {
      console.log('ExtensionItem - Toggling:', ext.name, 'Current enabled:', ext.enabled);
      await onToggle(ext);
    } catch (error) {
      console.error('Toggle failed:', error);
    }
  };

  const renderSubtitle = () => {
    const { description, command } = getSubtitle(extension);
    return (
      <>
        {description && <span>{description}</span>}
        {description && command && <br />}
        {command && <span className="font-mono text-xs">{command}</span>}
      </>
    );
  };

  // Bundled extensions and builtins are not editable
  const editable = !(extension.type === 'builtin' || extension.bundled);

  return (
    <div
      className="flex justify-between rounded-lg transition-colors border border-borderSubtle p-4 pt-3 hover:border-borderProminent hover:cursor-pointer"
      onClick={() => handleToggle(extension)}
    >
      <div className="flex flex-col w-max-[90%]">
        <h3 className="text-textStandard">{getFriendlyTitle(extension)}</h3>
        <p className="text-xs text-textSubtle">{renderSubtitle()}</p>
      </div>

      <div
        className="flex items-center justify-end gap-2 w-max-[10%]"
        onClick={(e) => e.stopPropagation()}
      >
        {!disableConfiguration && editable && onConfigure && (
          <button
            className="text-textSubtle hover:text-textStandard"
            onClick={() => onConfigure(extension)}
          >
            <Gear className="h-4 w-4" />
          </button>
        )}
        <Switch
          checked={extension.enabled}
          onCheckedChange={() => handleToggle(extension)}
          variant="mono"
        />
      </div>
    </div>
  );
}
