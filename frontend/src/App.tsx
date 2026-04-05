import React, { useState, useCallback, useEffect } from 'react';
import './index.css';
import StationList from './components/StationList';
import NowPlaying from './components/NowPlaying';
import Controls from './components/Controls';
import LogPanel from './components/LogPanel';
import CastModal from './components/CastModal';
import { useWebSocket } from './hooks/useWebSocket';
import { useTheme } from './hooks/useTheme';
import { api } from './hooks/useApi';
import {
  Station,
  StationMetadata,
  PlaybackStatus,
  CastDevice,
  ScanProgress,
} from './types';

function App() {
  const [stations, setStations] = useState<Station[]>([]);
  const [currentStation, setCurrentStation] = useState<Station | null>(null);
  const [metadata, setMetadata] = useState<StationMetadata | null>(null);
  const [playback, setPlayback] = useState<PlaybackStatus>({
    is_playing: false,
    station_id: null,
    station_name: null,
    volume: 0.75,
    elapsed_seconds: 0,
  });
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [showCastModal, setShowCastModal] = useState(false);
  const [castDevices, setCastDevices] = useState<CastDevice[]>([]);

  const { theme, setTheme } = useTheme();

  // Handle WebSocket messages
  const handleWsMessage = useCallback((data: any) => {
    if (data.type === 'initial_state' && data.data) {
      if (data.data.stations) setStations(data.data.stations);
      if (data.data.playback) setPlayback(data.data.playback);
      if (data.data.is_scanning) setIsScanning(data.data.is_scanning);
      if (data.data.cast_devices) setCastDevices(data.data.cast_devices);
      return;
    }

    switch (data.type) {
      case 'StationsUpdated':
        setStations(data.data || []);
        break;
      case 'MetadataUpdated':
        setMetadata(data.data);
        break;
      case 'ScanProgress':
        setScanProgress(data.data);
        setIsScanning(true);
        break;
      case 'PlaybackStatus':
        setPlayback(data.data);
        break;
      case 'CastDevices':
        setCastDevices(data.data || []);
        break;
      case 'Log':
        setLogs((prev) => [...prev.slice(-999), data.data]);
        break;
      case 'Error':
        setLogs((prev) => [...prev.slice(-999), `ERROR: ${data.data}`]);
        break;
    }
  }, []);

  const { isConnected } = useWebSocket(handleWsMessage);

  // Poll metadata for current station
  useEffect(() => {
    if (!currentStation) return;

    const interval = setInterval(async () => {
      try {
        const meta = await api.getMetadata(currentStation.id);
        if (meta) setMetadata(meta);
      } catch (e) {
        // ignore polling errors
      }
    }, 2000);

    return () => clearInterval(interval);
  }, [currentStation]);

  const handleScan = async () => {
    try {
      setIsScanning(true);
      await api.startScan(3);
    } catch (e: any) {
      setLogs((prev) => [...prev, `Scan error: ${e.message}`]);
      setIsScanning(false);
    }
  };

  const handleSelectStation = async (station: Station) => {
    try {
      setCurrentStation(station);
      await api.playStation(station.id);
      setPlayback((prev) => ({
        ...prev,
        is_playing: true,
        station_id: station.id,
        station_name: station.name,
      }));
    } catch (e: any) {
      setLogs((prev) => [...prev, `Play error: ${e.message}`]);
    }
  };

  const handlePlayPause = async () => {
    try {
      if (playback.is_playing) {
        await api.stopPlayback();
        setPlayback((prev) => ({ ...prev, is_playing: false }));
      } else if (currentStation) {
        await api.playStation(currentStation.id);
        setPlayback((prev) => ({ ...prev, is_playing: true }));
      }
    } catch (e: any) {
      setLogs((prev) => [...prev, `Playback error: ${e.message}`]);
    }
  };

  const handleVolumeChange = async (volume: number) => {
    setPlayback((prev) => ({ ...prev, volume }));
    try {
      await api.setVolume(volume);
    } catch (e) {
      // ignore
    }
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 overflow-hidden">
      {/* Connection status */}
      {!isConnected && (
        <div className="bg-amber-500 text-white text-xs text-center py-1 px-4">
          Connecting to backend...
        </div>
      )}

      {/* Scan progress bar */}
      {isScanning && scanProgress && (
        <div className="bg-blue-50 dark:bg-blue-900/20 px-4 py-2 flex items-center gap-3">
          <div className="flex-1 h-1.5 bg-blue-100 dark:bg-blue-900/50 rounded-full overflow-hidden">
            <div
              className="h-full bg-blue-500 rounded-full transition-all duration-500"
              style={{ width: `${scanProgress.percent_complete}%` }}
            />
          </div>
          <span className="text-xs text-blue-600 dark:text-blue-400 whitespace-nowrap">
            {scanProgress.current_block} &middot; Pass {scanProgress.current_pass}/
            {scanProgress.total_passes} &middot; {scanProgress.stations_found} stations
          </span>
        </div>
      )}

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* Left panel - Station list */}
        <div className="w-80 flex-shrink-0">
          <StationList
            stations={stations}
            currentStationId={currentStation?.id || null}
            onSelectStation={handleSelectStation}
            isScanning={isScanning}
            onScan={handleScan}
          />
        </div>

        {/* Main panel - Now Playing */}
        <NowPlaying
          station={currentStation}
          metadata={metadata}
          isPlaying={playback.is_playing}
        />

        {/* Log panel overlay */}
        <LogPanel
          logs={logs}
          isVisible={showLogs}
          onClose={() => setShowLogs(false)}
        />
      </div>

      {/* Bottom controls */}
      <Controls
        isPlaying={playback.is_playing}
        volume={playback.volume}
        stationName={playback.station_name}
        theme={theme}
        onPlayPause={handlePlayPause}
        onVolumeChange={handleVolumeChange}
        onThemeChange={setTheme}
        onToggleLogs={() => setShowLogs(!showLogs)}
        onCastClick={() => setShowCastModal(true)}
        showLogs={showLogs}
      />

      {/* Cast modal */}
      <CastModal
        isOpen={showCastModal}
        onClose={() => setShowCastModal(false)}
        devices={castDevices}
        onDevicesUpdate={setCastDevices}
      />
    </div>
  );
}

export default App;
