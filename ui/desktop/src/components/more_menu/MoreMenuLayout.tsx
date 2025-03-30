import MoreMenu from './MoreMenu';
import React from 'react';
import type { View } from '../../App';

export default function MoreMenuLayout({
  showMenu = true,
  setView,
  setIsGoosehintsModalOpen,
}: {
  showMenu?: boolean;
  setView?: (view: View, viewOptions?: Record<any, any>) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}) {
  return (
    <div className="relative flex items-center h-[36px] w-full" style={{ WebkitAppRegion: 'drag' }}>
      <div className="flex items-center h-full">
        {showMenu && (
          <div className="flex items-center justify-center h-full px-2 ml-2">
            <MoreMenu setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
          </div>
        )}
      </div>
    </div>
  );
}
