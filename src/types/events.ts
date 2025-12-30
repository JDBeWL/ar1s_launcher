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

// Minecraft 版本信息
export interface MinecraftVersion {
  id: string;
  type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha';
  url: string;
  time: string;
  releaseTime: string;
}

export interface VersionManifest {
  latest: {
    release: string;
    snapshot: string;
  };
  versions: MinecraftVersion[];
}

// 游戏实例
export interface GameInstance {
  id: string;
  name: string;
  version: string;
  path: string;
  createdTime?: string;
  loaderType?: string;
  gameVersion?: string;
  lastPlayed?: number;
  modLoader?: string;
  modLoaderVersion?: string;
  icon?: string;
}

// 创建实例参数
export interface CreateInstancePayload {
  newInstanceName: string;
  baseVersionId: string;
  loader?: LoaderPayload;
}

// 加载器参数（匹配后端 LoaderType）
export type LoaderPayload = 
  | { type: 'forge'; mc_version: string; loader_version: string }
  | { type: 'fabric'; mc_version: string; loader_version: string }
  | { type: 'quilt'; mc_version: string; loader_version: string }
  | { type: 'neoforge'; mc_version: string; loader_version: string };

// 安装进度事件
export interface InstallProgressPayload {
  progress: number;
  message: string;
  indeterminate: boolean;
}

// Forge 版本信息
export interface ForgeVersion {
  version: string;
  mcversion: string;
  build: number;
}

// 通用加载器版本信息
export interface LoaderVersionInfo {
  version: string;
  stable?: boolean;
}

// 可用加载器信息
export interface AvailableLoaders {
  forge: boolean;
  fabric: boolean;
  quilt: boolean;
  neoforge: boolean;
}

// 加载器类型
export type ModLoaderType = 'None' | 'Forge' | 'Fabric' | 'Quilt' | 'NeoForge';
