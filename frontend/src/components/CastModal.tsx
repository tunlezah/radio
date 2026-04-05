import React, { useEffect, useState } from 'react';
import { CastDevice } from '../types';
import { api } from '../hooks/useApi';

interface CastModalProps {
  isOpen: boolean;
  onClose: () => void;
  devices: CastDevice[];
  onDevicesUpdate: (devices: CastDevice[]) => void;
}

export default function CastModal({ isOpen, onClose, devices, onDevicesUpdate }: CastModalProps) {
  const [isDiscovering, setIsDiscovering] = useState(false);

  const handleDiscover = async () => {
    setIsDiscovering(true);
    try {
      const found = await api.discoverCastDevices();
      onDevicesUpdate(found);
    } catch (e) {
      console.error('Discovery failed:', e);
    }
    setIsDiscovering(false);
  };

  const handleCast = async (deviceId: string) => {
    try {
      await api.castToDevice(deviceId);
      onClose();
    } catch (e) {
      console.error('Cast failed:', e);
    }
  };

  const handleStopCast = async () => {
    try {
      await api.stopCasting();
    } catch (e) {
      console.error('Stop cast failed:', e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            Cast to Device
          </h3>
          <button
            onClick={onClose}
            className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 rounded transition-colors"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="p-5">
          {/* Discover button */}
          <button
            onClick={handleDiscover}
            disabled={isDiscovering}
            className="w-full py-2.5 px-4 bg-blue-500 hover:bg-blue-600 text-white rounded-xl text-sm font-medium transition-all disabled:opacity-50 mb-4"
          >
            {isDiscovering ? 'Discovering...' : 'Discover Devices'}
          </button>

          {/* Device list */}
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {devices.length === 0 ? (
              <div className="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">
                No cast devices found. Click Discover to search your network.
              </div>
            ) : (
              devices.map((device) => (
                <button
                  key={device.id}
                  onClick={() => handleCast(device.id)}
                  className="w-full flex items-center gap-3 p-3 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-700 border border-gray-200 dark:border-gray-600 transition-all"
                >
                  <div
                    className={`w-10 h-10 rounded-full flex items-center justify-center ${
                      device.device_type === 'chromecast'
                        ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-500'
                        : 'bg-purple-100 dark:bg-purple-900/30 text-purple-500'
                    }`}
                  >
                    {device.device_type === 'chromecast' ? (
                      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11zm20-7H3c-1.1 0-2 .9-2 2v3h2V5h18v14h-7v2h7c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
                      </svg>
                    ) : (
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.858 15.355-5.858 21.213 0" />
                      </svg>
                    )}
                  </div>
                  <div className="text-left flex-1">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">
                      {device.name}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 capitalize">
                      {device.device_type}
                    </div>
                  </div>
                  {device.is_connected && (
                    <span className="text-xs text-emerald-500 font-medium">Connected</span>
                  )}
                </button>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
