import React, { useEffect, useRef } from 'react';

interface LogPanelProps {
  logs: string[];
  isVisible: boolean;
  onClose: () => void;
}

export default function LogPanel({ logs, isVisible, onClose }: LogPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div
      className={`absolute bottom-0 left-0 right-0 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 shadow-2xl transition-transform duration-300 ease-out z-40 ${
        isVisible ? 'translate-y-0' : 'translate-y-full'
      }`}
      style={{ height: '40%' }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <svg
            className="w-4 h-4 text-gray-500 dark:text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            System Logs
          </span>
          <span className="text-xs text-gray-400 bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded-full">
            {logs.length}
          </span>
        </div>
        <button
          onClick={onClose}
          className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 rounded transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Log entries */}
      <div ref={scrollRef} className="overflow-y-auto h-full pb-12 px-4 py-2">
        {logs.length === 0 ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">
            No log entries yet.
          </div>
        ) : (
          logs.map((log, i) => (
            <div
              key={i}
              className={`py-1 font-mono text-xs ${
                log.includes('Error') || log.includes('error') || log.includes('FAIL')
                  ? 'text-red-500'
                  : log.includes('Warning') || log.includes('Weak')
                  ? 'text-amber-500'
                  : log.includes('complete') || log.includes('found') || log.includes('Playing')
                  ? 'text-emerald-500 dark:text-emerald-400'
                  : 'text-gray-600 dark:text-gray-400'
              }`}
            >
              {log}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
