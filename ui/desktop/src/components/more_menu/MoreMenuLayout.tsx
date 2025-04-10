import MoreMenu from './MoreMenu';
import type { View } from '../../App';

export default function MoreMenuLayout({
  hasMessages,
  showMenu = true,
  setView,
  setIsGoosehintsModalOpen,
}: {
  hasMessages?: boolean;
  showMenu?: boolean;
  setView?: (view: View, viewOptions?: Record<any, any>) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}) {
  return (
    <div
      className="relative flex items-center h-14 border-b w-full"
      style={{ WebkitAppRegion: 'drag' }}
    >
      {showMenu && (
        <div className="flex items-center justify-end h-full w-full">
          <MoreMenu setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
        </div>
      )}
    </div>
  );
}
