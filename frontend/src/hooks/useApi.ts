const API_BASE = process.env.REACT_APP_API_URL || `http://${window.location.hostname}:8080/api`;

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `API error: ${response.status}`);
  }

  return response.json();
}

export const api = {
  getStations: () => fetchApi<any[]>('/stations'),
  getStatus: () => fetchApi<any>('/status'),
  startScan: (passes?: number) =>
    fetchApi<any>('/scan', {
      method: 'POST',
      body: JSON.stringify(passes ? { passes } : null),
    }),
  playStation: (stationId: string) =>
    fetchApi<any>(`/play/${stationId}`, { method: 'POST' }),
  stopPlayback: () => fetchApi<any>('/stop', { method: 'POST' }),
  setVolume: (volume: number) =>
    fetchApi<any>('/volume', {
      method: 'POST',
      body: JSON.stringify({ volume }),
    }),
  getCastDevices: () => fetchApi<any[]>('/cast/devices'),
  discoverCastDevices: () =>
    fetchApi<any[]>('/cast/discover', { method: 'POST' }),
  castToDevice: (deviceId: string) =>
    fetchApi<any>(`/cast/${deviceId}`, { method: 'POST' }),
  stopCasting: () => fetchApi<any>('/cast/stop', { method: 'POST' }),
  getLogs: () => fetchApi<string[]>('/logs'),
  getMetadata: (stationId: string) => fetchApi<any>(`/stations/${stationId}/metadata`),
  systemCheck: () => fetchApi<any[]>('/system/check'),
};
