/**
 * 前端 API 服务层
 * 统一封装 Tauri invoke 调用，提供类型安全和请求去重
 */
import { invoke } from '@tauri-apps/api/core';
import { withCache, cache, CacheKeys } from './cache';
import type {
  VersionManifest,
  GameInstance,
  AvailableLoaders,
  ForgeVersion,
  LoaderVersionInfo,
  InstanceNameValidation,
  GameDirInfo,
  ModrinthVersion,
  ModrinthSearchResult,
  WindowSettings,
  AutoMemoryConfig,
  AuthStatus,
  DeviceCodeDisplay,
  MicrosoftLoginResult,
  VersionSizeInfo,
  JavaCompatibilityResult,
  GameConfig,
} from '../types/events';

// ============ 请求去重机制 ============

/** 进行中的请求缓存 */
const pendingRequests = new Map<string, Promise<unknown>>();

/**
 * 去重的 invoke 调用
 * 相同的请求在进行中时会复用同一个 Promise
 */
async function dedupedInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: { skipDedup?: boolean }
): Promise<T> {
  // 如果跳过去重，直接调用
  if (options?.skipDedup) {
    return invoke<T>(cmd, args);
  }

  // 生成请求唯一键
  const key = `${cmd}:${JSON.stringify(args ?? {})}`;

  // 检查是否有进行中的相同请求
  if (pendingRequests.has(key)) {
    return pendingRequests.get(key) as Promise<T>;
  }

  // 创建新请求
  const promise = invoke<T>(cmd, args).finally(() => {
    pendingRequests.delete(key);
  });

  pendingRequests.set(key, promise);
  return promise;
}

/**
 * 清除所有进行中的请求缓存
 * 用于特殊场景（如用户登出）
 */
export function clearPendingRequests(): void {
  pendingRequests.clear();
}

/**
 * 获取当前进行中的请求数量
 */
export function getPendingRequestCount(): number {
  return pendingRequests.size;
}

// ============ 版本相关 API ============

export const versionApi = {
  /** 获取 Minecraft 版本列表（缓存 5 分钟） */
  async getVersions(): Promise<VersionManifest> {
    return withCache(CacheKeys.VERSIONS, () =>
      dedupedInvoke<VersionManifest>('get_versions')
    );
  },

  /** 获取指定版本的文件大小信息 */
  async getVersionSize(versionId: string, mirror?: string): Promise<VersionSizeInfo> {
    return invoke<VersionSizeInfo>('get_version_size', { versionId, mirror });
  },

  /** 下载指定版本 */
  async downloadVersion(versionId: string, mirror?: string): Promise<void> {
    // 下载不去重，每次都是新请求
    return invoke('download_version', { versionId, mirror });
  },

  /** 取消下载 */
  async cancelDownload(): Promise<void> {
    return invoke('cancel_download');
  },

  /** 验证版本文件完整性 */
  async validateVersionFiles(versionId: string): Promise<string[]> {
    return dedupedInvoke<string[]>('validate_version_files', { versionId });
  },
};

// ============ 实例相关 API ============

export const instanceApi = {
  async getInstances(): Promise<GameInstance[]> {
    return dedupedInvoke<GameInstance[]>('get_instances');
  },

  async createInstance(
    newInstanceName: string,
    baseVersionId: string,
    loader?: { type: string; mc_version: string; loader_version: string }
  ): Promise<void> {
    await invoke('create_instance', { newInstanceName, baseVersionId, loader });
    cache.deleteByPrefix(CacheKeys.INSTANCES);
  },

  async deleteInstance(instanceName: string): Promise<void> {
    await invoke('delete_instance', { instanceName });
    cache.deleteByPrefix(CacheKeys.INSTANCES);
  },

  async renameInstance(oldName: string, newName: string): Promise<void> {
    await invoke('rename_instance', { oldName, newName });
    cache.deleteByPrefix(CacheKeys.INSTANCES);
  },

  async openInstanceFolder(instanceName: string): Promise<void> {
    return invoke('open_instance_folder', { instanceName });
  },

  async launchInstance(instanceName: string, overrideJavaPath?: string): Promise<void> {
    await invoke('launch_instance', { instanceName, overrideJavaPath });
    cache.deleteByPrefix(CacheKeys.INSTANCES);
  },

  /** 验证实例名称 */
  async validateInstanceName(name: string): Promise<InstanceNameValidation> {
    return dedupedInvoke<InstanceNameValidation>('validate_instance_name_cmd', { name });
  },

  /** 检查实例名称是否可用 */
  async checkInstanceNameAvailable(name: string): Promise<InstanceNameValidation> {
    return dedupedInvoke<InstanceNameValidation>('check_instance_name_available', { name });
  },
};

// ============ 加载器相关 API ============

export const loaderApi = {
  /** 获取可用的加载器类型（缓存 5 分钟） */
  async getAvailableLoaders(minecraftVersion: string): Promise<AvailableLoaders> {
    return withCache(`${CacheKeys.LOADERS}:${minecraftVersion}`, () =>
      dedupedInvoke<AvailableLoaders>('get_available_loaders', { minecraftVersion })
    );
  },

  /** 获取 Forge 版本列表（缓存 5 分钟） */
  async getForgeVersions(minecraftVersion: string): Promise<ForgeVersion[]> {
    return withCache(`${CacheKeys.LOADERS}:forge:${minecraftVersion}`, () =>
      dedupedInvoke<ForgeVersion[]>('get_forge_versions', { minecraftVersion })
    );
  },

  /** 获取 Fabric 版本列表（缓存 5 分钟） */
  async getFabricVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:fabric:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_fabric_versions', { minecraftVersion })
    );
  },

  /** 获取 Quilt 版本列表（缓存 5 分钟） */
  async getQuiltVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:quilt:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_quilt_versions', { minecraftVersion })
    );
  },

  /** 获取 NeoForge 版本列表（缓存 5 分钟） */
  async getNeoForgeVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:neoforge:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_neoforge_versions', { minecraftVersion })
    );
  },
};

// ============ Java 相关 API ============

export const javaApi = {
  /** 查找 Java 安装（缓存 10 分钟） */
  async findJavaInstallations(): Promise<string[]> {
    return withCache(CacheKeys.JAVA, () =>
      dedupedInvoke<string[]>('find_java_installations_command'), 10 * 60 * 1000
    );
  },

  /** 强制刷新 Java 安装列表 */
  async refreshJavaInstallations(): Promise<string[]> {
    cache.delete(CacheKeys.JAVA);
    return invoke<string[]>('refresh_java_installations');
  },

  /** 设置 Java 路径 */
  async setJavaPath(path: string): Promise<void> {
    return invoke('set_java_path_command', { path });
  },

  /** 验证 Java 路径 */
  async validateJavaPath(path: string): Promise<boolean> {
    return dedupedInvoke<boolean>('validate_java_path', { path });
  },

  /** 获取 Java 版本 */
  async getJavaVersion(path: string): Promise<string> {
    return dedupedInvoke<string>('get_java_version', { path });
  },

  /** 获取自定义 Java 路径列表 */
  async getCustomJavaPaths(): Promise<string[]> {
    return dedupedInvoke<string[]>('get_custom_java_paths');
  },

  /** 添加自定义 Java 路径 */
  async addCustomJavaPath(path: string): Promise<void> {
    return invoke('add_custom_java_path', { path });
  },

  /** 移除自定义 Java 路径 */
  async removeCustomJavaPath(path: string): Promise<void> {
    return invoke('remove_custom_java_path', { path });
  },

  /** 检查 Java 版本兼容性 */
  async checkJavaCompatibility(mcVersion: string): Promise<JavaCompatibilityResult> {
    return invoke<JavaCompatibilityResult>('check_java_compatibility', { mcVersion });
  },
};

// ============ 配置相关 API ============

export const configApi = {
  async getConfig(): Promise<GameConfig> {
    return dedupedInvoke<GameConfig>('get_config');
  },

  /** 获取游戏目录 */
  async getGameDir(): Promise<string> {
    return dedupedInvoke<string>('get_game_dir');
  },

  /** 设置游戏目录 */
  async setGameDir(path: string): Promise<void> {
    cache.deleteByPrefix(`${CacheKeys.CONFIG}:game_dir_info`);
    return invoke('set_game_dir', { path });
  },

  /** 获取游戏目录信息（缓存 2 分钟） */
  async getGameDirInfo(): Promise<GameDirInfo> {
    return withCache(`${CacheKeys.CONFIG}:game_dir_info`, () =>
      dedupedInvoke<GameDirInfo>('get_game_dir_info'), 2 * 60 * 1000
    );
  },

  /** 获取下载线程数 */
  async getDownloadThreads(): Promise<number> {
    return dedupedInvoke<number>('get_download_threads');
  },

  /** 设置下载线程数 */
  async setDownloadThreads(threads: number): Promise<void> {
    return invoke('set_download_threads', { threads });
  },

  /** 加载配置项 */
  async loadConfigKey(key: string): Promise<string | null> {
    return dedupedInvoke<string | null>('load_config_key', { key });
  },

  /** 保存配置项 */
  async saveConfigKey(key: string, value: string): Promise<void> {
    await invoke('save_config_key', { key, value });
    cache.deleteByPrefix(CacheKeys.CONFIG);
  },

  /** 获取上次选择的版本 */
  async getLastSelectedVersion(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_last_selected_version');
  },

  /** 设置上次选择的版本 */
  async setLastSelectedVersion(version: string): Promise<void> {
    return invoke('set_last_selected_version', { version });
  },

  /** 获取总内存（缓存 10 分钟） */
  async getTotalMemory(): Promise<number> {
    return withCache(`${CacheKeys.CONFIG}:total_memory`, () =>
      dedupedInvoke<number>('get_total_memory'), 10 * 60 * 1000
    );
  },

  /** 获取窗口设置 */
  async getWindowSettings(): Promise<WindowSettings> {
    return dedupedInvoke<WindowSettings>('get_window_settings');
  },

  /** 设置窗口设置 */
  async setWindowSettings(width: number | null, height: number | null, fullscreen: boolean): Promise<void> {
    return invoke('set_window_settings', { width, height, fullscreen });
  },

  /** 检查内存警告 */
  async checkMemoryWarning(memoryMb: number): Promise<string | null> {
    return dedupedInvoke<string | null>('check_memory_warning', { memoryMb });
  },

  /** 获取自动内存配置（缓存 10 分钟） */
  async getAutoMemoryConfig(): Promise<AutoMemoryConfig> {
    return withCache(`${CacheKeys.CONFIG}:auto_memory`, () =>
      dedupedInvoke<AutoMemoryConfig>('get_auto_memory_config'), 10 * 60 * 1000
    );
  },

  /** 设置自动内存开关 */
  async setAutoMemoryEnabled(enabled: boolean): Promise<void> {
    await invoke('set_auto_memory_enabled', { enabled });
    cache.deleteByPrefix(CacheKeys.CONFIG);
  },

  /** 自动设置内存 */
  async autoSetMemory(): Promise<number | null> {
    return invoke<number | null>('auto_set_memory');
  },

  /** 分析内存效率 */
  async analyzeMemoryEfficiency(memoryMb: number): Promise<string> {
    return dedupedInvoke<string>('analyze_memory_efficiency', { memoryMb });
  },
};

// ============ 用户相关 API ============

export const userApi = {
  /** 获取保存的用户名 */
  async getSavedUsername(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_saved_username');
  },

  /** 设置用户名 */
  async setSavedUsername(username: string): Promise<void> {
    return invoke('set_saved_username', { username });
  },

  /** 获取保存的 UUID */
  async getSavedUuid(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_saved_uuid');
  },

  /** 设置 UUID */
  async setSavedUuid(uuid: string): Promise<void> {
    return invoke('set_saved_uuid', { uuid });
  },

  /** 获取认证状态 */
  async getAuthStatus(): Promise<AuthStatus> {
    return dedupedInvoke<AuthStatus>('get_auth_status');
  },

  /** 开始 Microsoft 设备代码登录流程 */
  async startMicrosoftLogin(): Promise<DeviceCodeDisplay> {
    return invoke<DeviceCodeDisplay>('start_microsoft_login');
  },

  /** 完成 Microsoft 登录（轮询等待用户授权） */
  async completeMicrosoftLogin(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('complete_microsoft_login');
  },

  /** 开始 Microsoft Authorization Code + PKCE 登录流程，返回浏览器授权 URL */
  async startMicrosoftAuthCodeLogin(): Promise<string> {
    return invoke<string>('start_microsoft_auth_code_login');
  },

  /** 完成 Authorization Code 登录（等待浏览器回调） */
  async completeMicrosoftAuthCodeLogin(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('complete_microsoft_auth_code_login');
  },

  /** 刷新 Microsoft 认证 token */
  async refreshMicrosoftAuth(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('refresh_microsoft_auth');
  },

  /** 退出 Microsoft 登录 */
  async logoutMicrosoft(): Promise<void> {
    return invoke('logout_microsoft');
  },

  /** 设置认证类型 */
  async setAuthType(authType: string): Promise<void> {
    return invoke('set_auth_type', { authType });
  },
};

// ============ 启动器相关 API ============

export const launcherApi = {
  /** 启动 Minecraft */
  async launchMinecraft(options: {
    version: string;
    username: string;
    memory?: number;
    window_width?: number;
    window_height?: number;
    fullscreen?: boolean;
    auth_type?: string;
    uuid?: string;
    override_java_path?: string;
  }): Promise<void> {
    return invoke('launch_minecraft', { options });
  },
};

// ============ 整合包相关 API ============

export const modpackApi = {
  /** 搜索 Modrinth 整合包 */
  async searchModrinthModpacks(params: {
    query?: string;
    gameVersions?: string[];
    loaders?: string[];
    categories?: string[];
    limit?: number;
    offset?: number;
    sortBy?: string;
  }): Promise<ModrinthSearchResult> {
    return dedupedInvoke<ModrinthSearchResult>('search_modrinth_modpacks', params);
  },

  /** 获取整合包版本列表 */
  async getModrinthModpackVersions(
    projectId: string,
    gameVersions?: string[],
    loaders?: string[]
  ): Promise<ModrinthVersion[]> {
    return dedupedInvoke<ModrinthVersion[]>('get_modrinth_modpack_versions', {
      projectId,
      gameVersions,
      loaders,
    });
  },

  /** 安装整合包 */
  async installModrinthModpack(options: {
    modpack_id: string;
    version_id: string;
    instance_name: string;
    install_path: string;
  }): Promise<void> {
    return invoke('install_modrinth_modpack', { options });
  },

  /** 取消整合包安装 */
  async cancelModpackInstall(): Promise<void> {
    return invoke('cancel_modpack_install');
  },
};

// ============ 统一导出 ============

export const api = {
  version: versionApi,
  instance: instanceApi,
  loader: loaderApi,
  java: javaApi,
  config: configApi,
  user: userApi,
  launcher: launcherApi,
  modpack: modpackApi,
  
  // 工具函数
  clearPendingRequests,
  getPendingRequestCount,
};

export default api;
