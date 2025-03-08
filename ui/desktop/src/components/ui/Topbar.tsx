import React from 'react';

interface TopbarProps {
  children?: React.ReactNode;
}

const Topbar: React.FC<TopbarProps> = ({ children }) => {
  return (
    <div
      className="relative flex items-center h-[44px] w-full border-b border-borderSubtle"
      style={{ WebkitAppRegion: 'drag' }}
    >
      {children}
    </div>
  );
};

export default Topbar;
