import React, { useEffect, Suspense } from 'react';

import { platformService } from '@platform';

import GooseLogo from './components/GooseLogo';
import SuspenseLoader from './components/SuspenseLoader';
import ChatComponent from './components/ChatComponent';
import { ElectronPlatformService } from './services/platform/electron/PlatformService';

const App: React.FC = (): React.ReactElement => {
  useEffect(() => {
    console.log(
      'Platform service is Electron:',
      platformService instanceof ElectronPlatformService
    );
  }, []);

  const isElectron = platformService instanceof ElectronPlatformService;

  return (
    <Suspense fallback={<SuspenseLoader />}>
      <div className="h-screen flex flex-col">
        <div className="p-5">
          <div
            className={`absolute top-2.5 right-2.5 px-2 py-1 text-white rounded text-xs ${
              isElectron ? 'bg-green-500' : 'bg-blue-500'
            }`}
          >
            Running in: {isElectron ? 'Electron' : 'Web Browser'}
          </div>

          <div className="flex items-center gap-4">
            <GooseLogo />
            <h1 className="text-2xl font-bold text-textProminent">Goose v2</h1>
          </div>
        </div>
        
        <div className="flex-1">
          <ChatComponent />
        </div>
      </div>
    </Suspense>
  );
};

export default App;