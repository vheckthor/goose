import React from 'react';
import SplashPills from './SplashPills';
import GooseLogo from './GooseLogo';

export default function Splash({ append }) {
  return (
    <div className="h-full flex flex-col py-12 px-6">
      <div className="relative text-textStandard mb-4">
        <div className="w-min animate-[flyin_2s_var(--spring-easing)_forwards]">
          <GooseLogo />
        </div>
      </div>

      <div className="flex">
        <SplashPills append={append} />
      </div>
    </div>
  );
}
