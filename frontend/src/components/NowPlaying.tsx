import React, { useEffect, useState } from 'react';
import { Station, StationMetadata } from '../types';

interface NowPlayingProps {
  station: Station | null;
  metadata: StationMetadata | null;
  isPlaying: boolean;
}

export default function NowPlaying({ station, metadata, isPlaying }: NowPlayingProps) {
  const [dlsAnimKey, setDlsAnimKey] = useState(0);

  useEffect(() => {
    if (metadata?.dls) {
      setDlsAnimKey((k) => k + 1);
    }
  }, [metadata?.dls?.text]);

  if (!station) {
    return (
      <div className="flex-1 flex items-center justify-center bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800">
        <div className="text-center">
          <div className="w-24 h-24 mx-auto mb-6 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
            <svg
              className="w-12 h-12 text-gray-400 dark:text-gray-500"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
              />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-gray-400 dark:text-gray-500">
            Select a station to start listening
          </h2>
          <p className="mt-2 text-sm text-gray-400 dark:text-gray-500">
            Scan for stations or choose one from the list
          </p>
        </div>
      </div>
    );
  }

  const signalPercent = Math.round(station.signal_strength * 100);
  const signalColor =
    signalPercent > 60
      ? 'text-emerald-500'
      : signalPercent > 30
      ? 'text-amber-500'
      : 'text-red-500';

  return (
    <div className="flex-1 flex flex-col bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800 p-8">
      {/* Top - Station info */}
      <div className="mb-8">
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-4xl font-bold text-gray-900 dark:text-white tracking-tight">
              {station.name}
            </h1>
            <p className="mt-2 text-lg text-gray-500 dark:text-gray-400">
              {station.ensemble_name}
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className={`text-right ${signalColor}`}>
              <div className="text-2xl font-bold">{signalPercent}%</div>
              <div className="text-xs uppercase tracking-wider opacity-75">Signal</div>
            </div>
          </div>
        </div>

        {/* Station details chips */}
        <div className="flex flex-wrap gap-2 mt-4">
          <span className="px-3 py-1 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded-full text-xs font-medium">
            {station.codec}
          </span>
          <span className="px-3 py-1 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded-full text-xs font-medium">
            {station.bitrate} kbps
          </span>
          <span className="px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-full text-xs font-medium">
            {station.program_type}
          </span>
          <span className="px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-full text-xs font-medium">
            {(station.frequency / 1e6).toFixed(3)} MHz
          </span>
          <span className="px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-full text-xs font-medium">
            Block {station.block_name}
          </span>
        </div>
      </div>

      {/* Middle - Dynamic metadata */}
      <div className="flex-1 flex flex-col justify-center">
        {/* DLS Text */}
        {metadata?.dls && (
          <div className="mb-8">
            <div
              key={dlsAnimKey}
              className="text-2xl font-medium text-gray-700 dark:text-gray-200 animate-fade-in"
            >
              {metadata.dls.text}
            </div>
            <div className="mt-2 text-xs text-gray-400 dark:text-gray-500">
              Dynamic Label Segment
            </div>
          </div>
        )}

        {/* SLS Image */}
        {metadata?.sls && (
          <div className="mb-8">
            <div className="rounded-2xl overflow-hidden shadow-lg inline-block">
              <img
                src={`data:${metadata.sls.content_type};base64,${metadata.sls.image_data_base64}`}
                alt="Slideshow"
                className="max-h-48 object-contain"
              />
            </div>
          </div>
        )}

        {/* Playback visualization */}
        {isPlaying && (
          <div className="flex items-center gap-1 mt-4">
            {[...Array(32)].map((_, i) => (
              <div
                key={i}
                className="w-1.5 bg-blue-500 dark:bg-blue-400 rounded-full animate-eq-bar"
                style={{
                  animationDelay: `${i * 50}ms`,
                  height: `${8 + Math.random() * 24}px`,
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* Bottom - Signal quality */}
      <div className="mt-auto">
        <div className="grid grid-cols-3 gap-4">
          <div className="bg-white dark:bg-gray-800 rounded-2xl p-4 shadow-sm">
            <div className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-1">
              Signal Quality
            </div>
            <div className={`text-xl font-bold ${signalColor}`}>
              {signalPercent > 60 ? 'Excellent' : signalPercent > 30 ? 'Good' : 'Weak'}
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-2xl p-4 shadow-sm">
            <div className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-1">
              Bit Error Rate
            </div>
            <div className="text-xl font-bold text-gray-900 dark:text-white">
              {metadata ? `${(metadata.bit_error_rate * 100).toFixed(3)}%` : '--'}
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-2xl p-4 shadow-sm">
            <div className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-1">
              Audio Level
            </div>
            <div className="text-xl font-bold text-gray-900 dark:text-white">
              {metadata ? `${metadata.audio_level_db.toFixed(1)} dB` : '--'}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
