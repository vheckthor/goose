import MoreMenu from './MoreMenu';
import type { View } from '../../App';
import { Document } from '../icons';

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
        <div className="flex items-center justify-between w-full h-full">
          <div
            className="hover:cursor-pointer hover:text-textStandard flex items-center [&>svg]:size-4"
            onClick={async () => {
              if (hasMessages) {
                window.electron.directoryChooser();
              } else {
                window.electron.directoryChooser(true);
              }
            }}
          >
            <Document className="mr-1" />
            <div className="w-max-[200px] truncate [direction:rtl]">
              Working in {window.appConfig.get('GOOSE_WORKING_DIR')}
            </div>
          </div>
          <div className="flex items-center justify-end h-full w-full">
            <MoreMenu setView={setView} setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
          </div>
        </div>
      )}
    </div>
  );
}
