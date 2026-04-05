import React, { useState } from 'react';
import { CastDevice, Theme } from '../types';

interface ControlsProps {
  isPlaying: boolean;
  volume: number;
  stationName: string | null;
  theme: Theme;
  onPlayPause: () => void;
  onVolumeChange: (volume: number) => void;
  onThemeChange: (theme: Theme) => void;
  onToggleLogs: () => void;
  onCastClick: () => void;
  showLogs: boolean;
}

export default function Controls({
  isPlaying,
  volume,
  stationName,
  theme,
  onPlayPause,
  onVolumeChange,
  onThemeChange,
  onToggleLogs,
  onCastClick,
  showLogs,
}: ControlsProps) {
  const [showThemeMenu, setShowThemeMenu] = useState(false);

  return (
    <div className="bg-white/80 dark:bg-gray-900/80 backdrop-blur-xl border-t border-gray-200 dark:border-gray-700 px-6 py-3">
      <div className="flex items-center justify-between max-w-screen-2xl mx-auto">
        {/* Left - Now playing info */}
        <div className="flex items-center gap-4 min-w-0 flex-1">
          {stationName && (
            <div className="flex items-center gap-3 min-w-0">
              <div
                className={`w-10 h-10 rounded-xl flex items-center justify-center ${
                  isPlaying
                    ? 'bg-blue-500 shadow-lg shadow-blue-500/25'
                    : 'bg-gray-200 dark:bg-gray-700'
                }`}
              >
                {isPlaying ? (
                  <div className="flex gap-0.5 items-end h-4">
                    <div className="w-0.5 bg-white rounded animate-eq-bar" style={{ height: '40%', animationDelay: '0ms' }} />
                    <div className="w-0.5 bg-white rounded animate-eq-bar" style={{ height: '70%', animationDelay: '150ms' }} />
                    <div className="w-0.5 bg-white rounded animate-eq-bar" style={{ height: '50%', animationDelay: '300ms' }} />
                    <div className="w-0.5 bg-white rounded animate-eq-bar" style={{ height: '80%', animationDelay: '100ms' }} />
                  </div>
                ) : (
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19V6l12-3v13" />
                  </svg>
                )}
              </div>
              <div className="min-w-0">
                <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                  {stationName}
                </p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {isPlaying ? 'Now Playing' : 'Paused'}
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Center - Playback controls */}
        <div className="flex items-center gap-4">
          <button
            onClick={onPlayPause}
            disabled={!stationName}
            className={`w-12 h-12 rounded-full flex items-center justify-center transition-all ${
              stationName
                ? isPlaying
                  ? 'bg-blue-500 hover:bg-blue-600 text-white shadow-lg shadow-blue-500/25'
                  : 'bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-300 dark:text-gray-600 cursor-not-allowed'
            }`}
          >
            {isPlaying ? (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" />
              </svg>
            ) : (
              <svg className="w-5 h-5 ml-0.5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M8 5v14l11-7z" />
              </svg>
            )}
          </button>
        </div>

        {/* Right - Volume, Cast, Theme, Logs */}
        <div className="flex items-center gap-3 flex-1 justify-end">
          {/* Volume */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => onVolumeChange(volume > 0 ? 0 : 0.75)}
              className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
            >
              {volume === 0 ? (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
                </svg>
              ) : (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                </svg>
              )}
            </button>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={volume}
              onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
              className="w-24 h-1 bg-gray-200 dark:bg-gray-700 rounded-full appearance-none cursor-pointer accent-blue-500"
            />
          </div>

          {/* Cast */}
          <button
            onClick={onCastClick}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-all"
            title="Cast to device"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2 16.1A5 5 0 015.9 20M2 12.05A9 9 0 019.95 20M2 8V6a2 2 0 012-2h16a2 2 0 012 2v12a2 2 0 01-2 2h-6" />
              <circle cx="2" cy="20" r="0" fill="currentColor" />
            </svg>
          </button>

          {/* Theme toggle */}
          <div className="relative">
            <button
              onClick={() => setShowThemeMenu(!showThemeMenu)}
              className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-all"
              title="Theme"
            >
              {theme === 'dark' ? (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                </svg>
              ) : theme === 'light' ? (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                </svg>
              ) : (
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                </svg>
              )}
            </button>

            {showThemeMenu && (
              <div className="absolute bottom-full right-0 mb-2 bg-white dark:bg-gray-800 rounded-xl shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-50">
                {(['light', 'dark', 'system'] as Theme[]).map((t) => (
                  <button
                    key={t}
                    onClick={() => {
                      onThemeChange(t);
                      setShowThemeMenu(false);
                    }}
                    className={`w-full px-4 py-2 text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700 ${
                      theme === t
                        ? 'text-blue-500 font-medium'
                        : 'text-gray-700 dark:text-gray-300'
                    }`}
                  >
                    {t.charAt(0).toUpperCase() + t.slice(1)}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Logs toggle */}
          <button
            onClick={onToggleLogs}
            className={`p-2 rounded-lg transition-all ${
              showLogs
                ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-500'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800'
            }`}
            title="Toggle logs"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 10h16M4 14h16M4 18h16" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
