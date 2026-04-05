export interface Station {
  id: string;
  name: string;
  ensemble_name: string;
  ensemble_id: number;
  service_id: number;
  frequency: number;
  block_name: string;
  signal_strength: number;
  program_type: string;
  is_active: boolean;
  bitrate: number;
  codec: string;
  last_seen: string;
}

export interface DlsInfo {
  text: string;
  charset: number;
  updated_at: string;
}

export interface SlsInfo {
  content_type: string;
  image_data_base64: string;
  width: number;
  height: number;
  updated_at: string;
}

export interface StationMetadata {
  station_id: string;
  dls: DlsInfo | null;
  sls: SlsInfo | null;
  signal_quality: number;
  bit_error_rate: number;
  audio_level_db: number;
}

export interface PlaybackStatus {
  is_playing: boolean;
  station_id: string | null;
  station_name: string | null;
  volume: number;
  elapsed_seconds: number;
}

export interface CastDevice {
  id: string;
  name: string;
  device_type: 'chromecast' | 'airplay';
  is_connected: boolean;
}

export interface ScanProgress {
  current_block: string;
  current_frequency: number;
  blocks_scanned: number;
  total_blocks: number;
  current_pass: number;
  total_passes: number;
  stations_found: number;
  percent_complete: number;
}

export interface SystemCheck {
  name: string;
  status: 'Pass' | 'Fail' | 'Warning';
  message: string;
  required: boolean;
}

export type Theme = 'light' | 'dark' | 'system';

export interface WsMessage {
  type: string;
  data: any;
}
