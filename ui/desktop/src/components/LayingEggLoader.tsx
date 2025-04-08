import React, { useEffect, useState } from 'react';

export default function LayingEggLoader() {
  const [dots, setDots] = useState('');

  useEffect(() => {
    const interval = setInterval(() => {
      setDots((prev) => (prev.length >= 3 ? '' : prev + '.'));
    }, 500);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 px-6 py-4 rounded-lg shadow-lg">
        <p className="text-lg font-medium text-gray-900 dark:text-gray-100">laying an egg{dots}</p>
      </div>
    </div>
  );
}
