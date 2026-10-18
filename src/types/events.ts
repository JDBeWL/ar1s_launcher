/**
 * 前后端事件与命令载荷类型
 * 字段命名按后端对应结构保持 snake_case 或 camelCase
 */
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

// 对应后端 MemoryStats，snake_case
export interface MemoryStats {
  total_memory_mb: number;
  used_memory_mb: number;
  available_memory_mb: number;
  memory_usage_percent: number;
}

// 对应后端 InstanceInfo，camelCase
export interface GameInstance {
  id: string;
  name: string;
  version: string;
  path: string;
  createdTime?: string;
  loaderType?: string;
  loaderVersion?: string;
  gameVersion?: string;
  lastPlayed?: number;
  modCount?: number;
  /** 整合包自带封面，没有时回退到加载器图标 */
  iconUrl?: string;
  /** 自动内存开启时，该实例启动预计分配的堆大小（MB） */
  autoMemoryMb?: number;
}

export interface ModInfo {
  /** 磁盘上的实际文件名，禁用时带 .disabled 后缀 */
  fileName: string;
  displayName: string;
  version?: string | null;
  enabled: boolean;
  size: number;
}

export interface CreateInstancePayload {
  newInstanceName: string;
  baseVersionId: string;
  loader?: LoaderPayload;
}

// 匹配后端 LoaderType
export type LoaderPayload =
  | { type: 'forge'; mc_version: string; loader_version: string }
  | { type: 'fabric'; mc_version: string; loader_version: string }
  | { type: 'quilt'; mc_version: string; loader_version: string }
  | { type: 'neoforge'; mc_version: string; loader_version: string };

export interface InstallProgressPayload {
  progress: number;
  message: string;
  indeterminate: boolean;
}

export interface ModpackActiveFile {
  path: string;
  downloaded: number;
  total: number;
}

// 对应后端 ModpackFilesProgress，只包含当前正在下载的文件
export interface ModpackFilesProgress {
  files: ModpackActiveFile[];
  completed_files: number;
  total_files: number;
  downloaded_bytes: number;
  total_bytes: number;
  speed: number;
}

export interface ForgeVersion {
  version: string;
  mcversion: string;
  build: number;
}

export interface LoaderVersionInfo {
  version: string;
  stable?: boolean;
}

export interface AvailableLoaders {
  forge: boolean;
  fabric: boolean;
  quilt: boolean;
  neoforge: boolean;
}

export type ModLoaderType = 'None' | 'Forge' | 'Fabric' | 'Quilt' | 'NeoForge';

export interface InstanceNameValidation {
  is_valid: boolean;
  error_message: string | null;
}

// 对应后端 GameDirInfo，snake_case
export interface GameDirInfo {
  path: string;
  versions: string[];
  total_size: number;
}

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

export interface ModrinthSearchResult {
  hits: ModrinthModpack[];
  total_hits: number;
}

// 对应后端 ModrinthModpackVersion，snake_case
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

export interface WindowSettings {
  width: number | null;
  height: number | null;
  fullscreen: boolean;
}

// 对应后端 GcStrategy，lowercase
export type GcStrategy = 'g1' | 'default' | 'parallel' | 'serial' | 'z';

// 对应后端 JvmAdvancedOptions，camelCase
export interface JvmAdvancedOptions {
  gcStrategy: GcStrategy;
  alwaysPreTouch: boolean;
  disableExplicitGc: boolean;
  stringDeduplication: boolean;
  perfDisableSharedMem: boolean;
  metaspaceSizeMb: number | null;
  metaspaceMaxSizeMb: number | null;
  heapDumpOnOom: boolean;
  extraArgs: string;
}

export interface AuthStatus {
  authType: 'offline' | 'microsoft';
  loggedIn: boolean;
  username: string | null;
  uuid: string | null;
  skinUrl: string | null;
  expiresAt: number | null;
}

// 刻意不含敏感的 device_code
export interface DeviceCodeDisplay {
  userCode: string;
  verificationUri: string;
  expiresIn: number;
}

// 不含 access_token 和 refresh_token
export interface MicrosoftLoginResult {
  username: string;
  uuid: string;
  skinUrl: string | null;
  expiresAt: number;
}

export interface JavaCompatibilityResult {
  compatible: boolean;
  hardBlock: boolean;
  diagnostic: string;
  currentJavaVersion: number | null;
  requiredJavaVersion: number;
  currentJavaPath: string | null;
  recommendedJavaPath: string | null;
  recommendedJavaVersion: number | null;
  /** 推荐项是否正好满足判据（被模组抬高下限后仍算满足） */
  recommendedJavaOptimal: boolean;
  /** 只能退而求其次时的说明，例如本机缺声明版本、或模组把下限抬高了 */
  recommendedJavaWarning: string | null;
  autoMatchEnabled: boolean;
}

export type ProxyMode = 'system' | 'direct' | 'custom';

export interface ProxySettings {
  mode: ProxyMode;
  url: string | null;
}

export interface ProxyTestResult {
  latencyMs: number;
  status: number;
}

export interface GameConfig {
  game_dir: string;
  version_isolation: boolean;
  java_path: string | null;
  custom_java_paths: string[];
  auto_match_java: boolean;
  download_threads: number;
  language: string | null;
  username: string | null;
  uuid: string | null;
  skin_url: string | null;
  max_memory: number;
  download_mirror: string | null;
  proxy_mode: ProxyMode;
  proxy_url: string | null;
  auto_memory_enabled: boolean;
  window_width: number | null;
  window_height: number | null;
  fullscreen: boolean;
  last_selected_version: string | null;
  auth_type: 'offline' | 'microsoft';
}

/** 对应后端 crash_diagnosis，camelCase */
export type CrashConfidence = 'high' | 'medium';

export type CrashFindingKind =
  | 'mixin'
  | 'classReaderTooOld'
  | 'javaVersion'
  | 'classVersion'
  | 'mainClass'
  | 'missingClass'
  | 'memory'
  | 'natives'
  | 'jvmCrash'
  | 'securityManager'
  | 'unknown';

export interface CrashEvidence {
  lineNo: number;
  text: string;
}

export interface CrashFinding {
  kind: CrashFindingKind;
  /** root = 根因，symptom = 被根因带出来的表象 */
  role: 'root' | 'symptom';
  conclusion: string;
  confidence: CrashConfidence;
  /** 一句话摘要，弹窗顶部只用它 */
  summary: string;
  /** 「问题模组」「问题类」这类人话标签，与 culprits 配套 */
  culpritLabel: string;
  /** 能定位到的 jar 文件名或类名 */
  culprits: string[];
  evidence: CrashEvidence[];
  suggestion: string;
}

export interface CrashContextLine {
  label: string;
  value: string;
}

export interface CrashSuggestedJava {
  path: string;
  version: number;
}

export interface CrashDiagnosis {
  exitCode: number | null;
  headline: string;
  recognized: boolean;
  findings: CrashFinding[];
  context: CrashContextLine[];
  excerpt: CrashEvidence[];
  rawOutput: string;
  /** 复制给上游用的纯文本，与弹窗同源 */
  reportText: string;
  /** 实例 id，供「换 Java 重试」直接重启 */
  instance: string;
  /** 判据允许、且不是本次用过的那份 Java */
  suggestedJava: CrashSuggestedJava | null;
}
