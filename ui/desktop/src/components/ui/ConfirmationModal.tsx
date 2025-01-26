import { Button } from './button';
import React from 'react';

export function ConfirmationModal({
  title,
  message,
  onConfirm,
  onCancel,
  confirmLabel = 'Yes',
  cancelLabel = 'No',
}) {
  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999]">
      <div className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-200 dark:border-gray-700">
        <div className="p-6">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100 mb-4">{title}</h2>
          <p className="text-sm text-gray-600 dark:text-gray-300 mb-6">{message}</p>
          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={onCancel} className="rounded-full px-4">
              {cancelLabel}
            </Button>
            <Button
              variant="primary"
              onClick={onConfirm}
              className="rounded-full px-4 bg-blue-600 hover:bg-blue-700 dark:bg-blue-600 dark:hover:bg-blue-700"
            >
              {confirmLabel}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
