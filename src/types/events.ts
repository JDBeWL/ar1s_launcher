export type DownloadStatus = 'downloading' | 'completed' | 'cancelled' | 'error';

export interface DownloadProgress {
  bytes_downloaded: number;
  total_bytes: number;
  speed: number;
  status: DownloadStatus;
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

export interface VersionSizeInfo {
  version_id: string;
  client_size: number;
  total_size: number;
}

// 游戏实例 (对应后端 InstanceInfo，使用 camelCase)
export interface GameInstance {
  id: string;
  name: string;
  version: string;
  path: string;
  createdTime?: string;
  loaderType?: string;
  gameVersion?: string;
  lastPlayed?: number;
  modCount?: number;
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

// 实例名称验证结果
export interface InstanceNameValidation {
  is_valid: boolean;
  error_message: string | null;
}

// 游戏目录信息 (对应后端 GameDirInfo，使用 snake_case)
export interface GameDirInfo {
  path: string;
  versions: string[];
  total_size: number;
}

// Modrinth 整合包搜索结果条目
export interface ModrinthModpack {
  slug: string;
  title: string;
  author: string;
  downloads: number;
  game_versions: string[];
  loaders: string[];
  description: string;
  icon_url?: string;
  date_created: string;
  date_modified: string;
  latest_version: string;
  categories: string[];
}

// Modrinth 整合包搜索结果
export interface ModrinthSearchResult {
  hits: ModrinthModpack[];
  total_hits: number;
}

// Modrinth 整合包版本 (对应后端 ModrinthModpackVersion，使用 snake_case)
export interface ModrinthVersion {
  id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  featured: boolean;
  date_published: string;
  downloads: number;
  files: ModrinthFile[];
  dependencies: ModrinthDependency[];
}

export interface ModrinthFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: {
    sha1: string;
    sha512: string;
  };
}

export interface ModrinthDependency {
  version_id?: string;
  project_id?: string;
  dependency_type: string;
}

// 窗口设置
export interface WindowSettings {
  width: number | null;
  height: number | null;
  fullscreen: boolean;
}

// 自动内存配置 (对应后端 AutoMemoryConfig，使用 snake_case)
export interface AutoMemoryConfig {
  enabled: boolean;
  max_limit_mb: number;
  safety_margin_percent: number;
}

// 认证状态
export interface AuthStatus {
  authType: 'offline' | 'microsoft';
  loggedIn: boolean;
  username: string | null;
  uuid: string | null;
  accessToken: string | null;
  expiresAt: number | null;
}

// Microsoft 登录设备代码信息
export interface DeviceCodeInfo {
  userCode: string;
  deviceCode: string;
  verificationUri: string;
  interval: number;
  expiresIn: number;
}

// Microsoft 认证结果
export interface MicrosoftAuthResult {
  username: string;
  uuid: string;
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
}