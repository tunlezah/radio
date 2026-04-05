import React, { useState, useMemo } from 'react';
import { Station } from '../types';

interface StationListProps {
  stations: Station[];
  currentStationId: string | null;
  onSelectStation: (station: Station) => void;
  isScanning: boolean;
  onScan: () => void;
}

function SignalIndicator({ strength }: { strength: number }) {
  const bars = Math.ceil(strength * 4);
  return (
    <div className="flex items-end gap-0.5 h-4">
      {[1, 2, 3, 4].map((level) => (
        <div
          key={level}
          className={`w-1 rounded-sm transition-colors ${
            level <= bars
              ? strength > 0.6
                ? 'bg-emerald-400'
                : strength > 0.3
                ? 'bg-amber-400'
                : 'bg-red-400'
              : 'bg-gray-300 dark:bg-gray-600'
          }`}
          style={{ height: `${level * 25}%` }}
        />
      ))}
    </div>
  );
}

export default function StationList({
  stations,
  currentStationId,
  onSelectStation,
  isScanning,
  onScan,
}: StationListProps) {
  const [search, setSearch] = useState('');
  const [filterEnsemble, setFilterEnsemble] = useState<string>('all');

  const ensembles = useMemo(() => {
    const unique = Array.from(new Set(stations.map((s) => s.ensemble_name)));
    return unique.sort();
  }, [stations]);

  const filteredStations = useMemo(() => {
    return stations.filter((s) => {
      const matchesSearch =
        search === '' ||
        s.name.toLowerCase().includes(search.toLowerCase()) ||
        s.ensemble_name.toLowerCase().includes(search.toLowerCase());
      const matchesEnsemble =
        filterEnsemble === 'all' || s.ensemble_name === filterEnsemble;
      return matchesSearch && matchesEnsemble;
    });
  }, [stations, search, filterEnsemble]);

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700">
      {/* Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Stations
          </h2>
          <span className="text-xs text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded-full">
            {stations.length}
          </span>
        </div>

        {/* Search */}
        <div className="relative mb-2">
          <svg
            className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            placeholder="Search stations..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-10 pr-4 py-2 text-sm rounded-xl border border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 transition-all"
          />
        </div>

        {/* Ensemble filter */}
        {ensembles.length > 1 && (
          <select
            value={filterEnsemble}
            onChange={(e) => setFilterEnsemble(e.target.value)}
            className="w-full text-xs py-1.5 px-2 rounded-lg border border-gray-200 dark:border-gray-600 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
          >
            <option value="all">All Ensembles</option>
            {ensembles.map((e) => (
              <option key={e} value={e}>
                {e}
              </option>
            ))}
          </select>
        )}
      </div>

      {/* Scan button */}
      <div className="px-4 py-2">
        <button
          onClick={onScan}
          disabled={isScanning}
          className={`w-full py-2 px-4 rounded-xl text-sm font-medium transition-all ${
            isScanning
              ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-500 cursor-wait'
              : 'bg-blue-500 hover:bg-blue-600 text-white shadow-sm hover:shadow-md'
          }`}
        >
          {isScanning ? (
            <span className="flex items-center justify-center gap-2">
              <svg
                className="animate-spin h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                />
              </svg>
              Scanning...
            </span>
          ) : (
            'Scan DAB+ Stations'
          )}
        </button>
      </div>

      {/* Station list */}
      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {filteredStations.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">
            <svg
              className="mx-auto h-12 w-12 mb-3 opacity-50"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
              />
            </svg>
            <p className="text-sm">
              {stations.length === 0
                ? 'No stations found. Click Scan to discover DAB+ stations.'
                : 'No matching stations.'}
            </p>
          </div>
        ) : (
          filteredStations.map((station) => (
            <button
              key={station.id}
              onClick={() => onSelectStation(station)}
              className={`w-full text-left p-3 rounded-xl mb-1 transition-all ${
                station.id === currentStationId
                  ? 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 shadow-sm'
                  : 'hover:bg-gray-50 dark:hover:bg-gray-800 border border-transparent'
              }`}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    {station.id === currentStationId && (
                      <span className="flex h-2 w-2">
                        <span className="animate-ping absolute inline-flex h-2 w-2 rounded-full bg-blue-400 opacity-75" />
                        <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500" />
                      </span>
                    )}
                    <span
                      className={`text-sm font-medium truncate ${
                        station.id === currentStationId
                          ? 'text-blue-700 dark:text-blue-300'
                          : 'text-gray-900 dark:text-white'
                      }`}
                    >
                      {station.name}
                    </span>
                  </div>
                  <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">
                    {station.ensemble_name} &middot; {station.block_name} &middot;{' '}
                    {station.bitrate}kbps
                  </p>
                </div>
                <SignalIndicator strength={station.signal_strength} />
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
