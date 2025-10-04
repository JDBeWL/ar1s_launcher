export type DownloadStatus = 'downloading' | 'completed' | 'cancelled' | 'error';

export interface DownloadProgress {
  progress: number;
  total: number;
  speed: number;
  status: DownloadStatus;
  bytes_downloaded: number;
  total_bytes: number;
  percent: number;
  error?: string;
}

export interface GameExitPayload {
    code: number | null;
    signal: string | null;
}

export type LaunchCommandPayload = string;
