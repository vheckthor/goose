import React from 'react';
import SplashPills from './SplashPills';
import GooseLogo from './GooseLogo';

interface SplashProps {
  append: (text: string) => void;
  activities: string[] | null;
  title?: string; // Add this
}

export default function Splash({ append, activities, title }: SplashProps) {
  return (
    <div className="flex flex-col items-center justify-center h-full">
      {title && (
        <div className="mb-4 text-2xl font-bold text-textStandard flex items-center">
          <span className="mr-2">✨</span>
          {title}
          <span className="ml-2">✨</span>
        </div>
      )}
      <div className="h-full flex flex-col pb-12">
        <div className="p-8">
          <div className="relative text-textStandard mb-12">
            <div className="w-min animate-[flyin_2s_var(--spring-easing)_forwards]">
              <GooseLogo />
            </div>
          </div>

          <div className="flex">
            <SplashPills append={append} activities={activities} />
          </div>
        </div>
      </div>
    </div>
  );
}
