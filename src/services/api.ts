/**
 * Tauri 后端接口调用模块
 * 按域封装 invoke，相同参数的并发请求合并，只读结果走缓存、写操作后按前缀失效
 */
import { invoke } from '@tauri-apps/api/core';
import { withCache, cache, CacheKeys } from './cache';
import type {
  VersionManifest,
  GameInstance,
  ModInfo,
  AvailableLoaders,
  ForgeVersion,
  LoaderVersionInfo,
  InstanceNameValidation,
  GameDirInfo,
  ModrinthVersion,
  ModrinthSearchResult,
  WindowSettings,
  JvmAdvancedOptions,
  AutoMemoryConfig,
  AuthStatus,
  DeviceCodeDisplay,
  MicrosoftLoginResult,
  VersionSizeInfo,
  JavaCompatibilityResult,
  GameConfig,
} from '../types/events';

const pendingRequests = new Map<string, Promise<unknown>>();

async function dedupedInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: { skipDedup?: boolean },
): Promise<T> {
  if (options?.skipDedup) {
    return invoke<T>(cmd, args);
  }

  const key = `${cmd}:${JSON.stringify(args ?? {})}`;
  if (pendingRequests.has(key)) {
    return pendingRequests.get(key) as Promise<T>;
  }

  const promise = invoke<T>(cmd, args).finally(() => {
    pendingRequests.delete(key);
  });

  pendingRequests.set(key, promise);
  return promise;
}

export const versionApi = {
  /** 默认 5 分钟缓存 */
  async getVersions(): Promise<VersionManifest> {
    return withCache(CacheKeys.VERSIONS, () => dedupedInvoke<VersionManifest>('get_versions'));
  },

  async getVersionSize(versionId: string, mirror?: string): Promise<VersionSizeInfo> {
    return invoke<VersionSizeInfo>('get_version_size', { versionId, mirror });
  },

  async downloadVersion(versionId: string, mirror?: string): Promise<void> {
    // 下载不去重，每次都是新请求
    return invoke('download_version', { versionId, mirror });
  },

  async cancelDownload(): Promise<void> {
    return invoke('cancel_download');
  },

  async validateVersionFiles(versionId: string): Promise<string[]> {
    return dedupedInvoke<string[]>('validate_version_files', { versionId });
  },
};

export const instanceApi = {
  async getInstances(): Promise<GameInstance[]> {
    return dedupedInvoke<GameInstance[]>('get_instances');
  },

  async createInstance(
    newInstanceName: string,
    baseVersionId: string,
    loader?: { type: string; mc_version: string; loader_version: string },
  ): Promise<void> {
    await invoke('create_instance', { newInstanceName, baseVersionId, loader });
  },

  async deleteInstance(instanceName: string): Promise<void> {
    await invoke('delete_instance', { instanceName });
  },

  async renameInstance(oldName: string, newName: string): Promise<void> {
    await invoke('rename_instance', { oldName, newName });
  },

  async openInstanceFolder(instanceName: string): Promise<void> {
    return invoke('open_instance_folder', { instanceName });
  },

  /** mods 目录里的模组，含已禁用的；状态会变，不走缓存 */
  async getMods(instanceName: string): Promise<ModInfo[]> {
    return invoke<ModInfo[]>('get_instance_mods', { instanceName });
  },

  /** 启用/禁用模组（改名 *.jar <-> *.jar.disabled） */
  async setModEnabled(instanceName: string, fileName: string, enabled: boolean): Promise<void> {
    await invoke('set_mod_enabled', { instanceName, fileName, enabled });
  },

  async deleteMod(instanceName: string, fileName: string): Promise<void> {
    await invoke('delete_instance_mod', { instanceName, fileName });
  },

  async launchInstance(instanceName: string, overrideJavaPath?: string): Promise<void> {
    await invoke('launch_instance', { instanceName, overrideJavaPath });
  },

  async validateInstanceName(name: string): Promise<InstanceNameValidation> {
    return dedupedInvoke<InstanceNameValidation>('validate_instance_name_cmd', { name });
  },

  async checkInstanceNameAvailable(name: string): Promise<InstanceNameValidation> {
    return dedupedInvoke<InstanceNameValidation>('check_instance_name_available', { name });
  },
};

export const loaderApi = {
  /** 默认 5 分钟缓存 */
  async getAvailableLoaders(minecraftVersion: string): Promise<AvailableLoaders> {
    return withCache(`${CacheKeys.LOADERS}:${minecraftVersion}`, () =>
      dedupedInvoke<AvailableLoaders>('get_available_loaders', { minecraftVersion }),
    );
  },

  /** 默认 5 分钟缓存 */
  async getForgeVersions(minecraftVersion: string): Promise<ForgeVersion[]> {
    return withCache(`${CacheKeys.LOADERS}:forge:${minecraftVersion}`, () =>
      dedupedInvoke<ForgeVersion[]>('get_forge_versions', { minecraftVersion }),
    );
  },

  /** 默认 5 分钟缓存 */
  async getFabricVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:fabric:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_fabric_versions', { minecraftVersion }),
    );
  },

  /** 默认 5 分钟缓存 */
  async getQuiltVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:quilt:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_quilt_versions', { minecraftVersion }),
    );
  },

  /** 默认 5 分钟缓存 */
  async getNeoForgeVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return withCache(`${CacheKeys.LOADERS}:neoforge:${minecraftVersion}`, () =>
      dedupedInvoke<LoaderVersionInfo[]>('get_neoforge_versions', { minecraftVersion }),
    );
  },
};

export const javaApi = {
  async findJavaInstallations(): Promise<string[]> {
    return withCache(CacheKeys.JAVA, () => dedupedInvoke<string[]>('find_java_installations_command'), 10 * 60 * 1000);
  },

  async refreshJavaInstallations(): Promise<string[]> {
    const result = await invoke<string[]>('refresh_java_installations');
    // 必须等后端刷完再失效，否则并发的 findJavaInstallations 会把旧列表写回缓存
    cache.delete(CacheKeys.JAVA);
    return result;
  },

  async setJavaPath(path: string): Promise<void> {
    return invoke('set_java_path_command', { path });
  },

  async validateJavaPath(path: string): Promise<boolean> {
    return dedupedInvoke<boolean>('validate_java_path', { path });
  },

  async getJavaVersion(path: string): Promise<string> {
    return dedupedInvoke<string>('get_java_version', { path });
  },

  async getCustomJavaPaths(): Promise<string[]> {
    return dedupedInvoke<string[]>('get_custom_java_paths');
  },

  async addCustomJavaPath(path: string): Promise<void> {
    return invoke('add_custom_java_path', { path });
  },

  async removeCustomJavaPath(path: string): Promise<void> {
    return invoke('remove_custom_java_path', { path });
  },

  async checkJavaCompatibility(mcVersion: string): Promise<JavaCompatibilityResult> {
    return invoke<JavaCompatibilityResult>('check_java_compatibility', { mcVersion });
  },
};

export const configApi = {
  async getConfig(): Promise<GameConfig> {
    return dedupedInvoke<GameConfig>('get_config');
  },

  async getGameDir(): Promise<string> {
    return dedupedInvoke<string>('get_game_dir');
  },

  async setGameDir(path: string): Promise<void> {
    await invoke('set_game_dir', { path });
    cache.deleteByPrefix(`${CacheKeys.CONFIG}:game_dir_info`);
  },

  async getGameDirInfo(): Promise<GameDirInfo> {
    return withCache(
      `${CacheKeys.CONFIG}:game_dir_info`,
      () => dedupedInvoke<GameDirInfo>('get_game_dir_info'),
      2 * 60 * 1000,
    );
  },

  async getDownloadThreads(): Promise<number> {
    return dedupedInvoke<number>('get_download_threads');
  },

  async setDownloadThreads(threads: number): Promise<void> {
    return invoke('set_download_threads', { threads });
  },

  async loadConfigKey(key: string): Promise<string | null> {
    return dedupedInvoke<string | null>('load_config_key', { key });
  },

  async saveConfigKey(key: string, value: string): Promise<void> {
    await invoke('save_config_key', { key, value });
    cache.deleteByPrefix(CacheKeys.CONFIG);
  },

  async getLastSelectedVersion(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_last_selected_version');
  },

  async setLastSelectedVersion(version: string): Promise<void> {
    return invoke('set_last_selected_version', { version });
  },

  async getTotalMemory(): Promise<number> {
    const key = `${CacheKeys.CONFIG}:total_memory`;
    const read = () => dedupedInvoke<number>('get_total_memory');

    let mb = await withCache<number>(key, read, 10 * 60 * 1000);
    if (!mb) {
      // sysinfo 冷启动首次刷新可能返回 0，缓存住会让之后 10 分钟都读到 0
      cache.delete(key);
      mb = await read();
      if (mb) cache.set(key, mb, 10 * 60 * 1000);
    }
    return mb;
  },

  async getWindowSettings(): Promise<WindowSettings> {
    return dedupedInvoke<WindowSettings>('get_window_settings');
  },

  async setWindowSettings(width: number | null, height: number | null, fullscreen: boolean): Promise<void> {
    return invoke('set_window_settings', { width, height, fullscreen });
  },

  async getJvmAdvancedOptions(): Promise<JvmAdvancedOptions> {
    return invoke<JvmAdvancedOptions>('get_jvm_advanced_options');
  },

  async setJvmAdvancedOptions(options: JvmAdvancedOptions): Promise<void> {
    return invoke('set_jvm_advanced_options', { options });
  },

  async getEffectiveJvmArgs(memoryMb?: number): Promise<string[]> {
    return invoke<string[]>('get_effective_jvm_args', { memoryMb: memoryMb ?? null });
  },

  async checkMemoryWarning(memoryMb: number): Promise<string | null> {
    return dedupedInvoke<string | null>('check_memory_warning', { memoryMb });
  },

  async getAutoMemoryConfig(): Promise<AutoMemoryConfig> {
    return withCache(
      `${CacheKeys.CONFIG}:auto_memory`,
      () => dedupedInvoke<AutoMemoryConfig>('get_auto_memory_config'),
      10 * 60 * 1000,
    );
  },

  async setAutoMemoryEnabled(enabled: boolean): Promise<void> {
    await invoke('set_auto_memory_enabled', { enabled });
    cache.deleteByPrefix(CacheKeys.CONFIG);
  },

  async autoSetMemory(): Promise<number | null> {
    return invoke<number | null>('auto_set_memory');
  },

  async analyzeMemoryEfficiency(memoryMb: number): Promise<string> {
    return dedupedInvoke<string>('analyze_memory_efficiency', { memoryMb });
  },
};

export const userApi = {
  async getSavedUsername(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_saved_username');
  },

  async setSavedUsername(username: string): Promise<void> {
    return invoke('set_saved_username', { username });
  },

  async getSavedUuid(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_saved_uuid');
  },

  async setSavedUuid(uuid: string): Promise<void> {
    return invoke('set_saved_uuid', { uuid });
  },

  async getAuthStatus(): Promise<AuthStatus> {
    return dedupedInvoke<AuthStatus>('get_auth_status');
  },

  async startMicrosoftLogin(): Promise<DeviceCodeDisplay> {
    return invoke<DeviceCodeDisplay>('start_microsoft_login');
  },

  /** 轮询等待用户授权 */
  async completeMicrosoftLogin(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('complete_microsoft_login');
  },

  /** Authorization Code + PKCE 流程，返回浏览器授权 URL */
  async startMicrosoftAuthCodeLogin(): Promise<string> {
    return invoke<string>('start_microsoft_auth_code_login');
  },

  /** 等浏览器回调，而非轮询 */
  async completeMicrosoftAuthCodeLogin(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('complete_microsoft_auth_code_login');
  },

  async refreshMicrosoftAuth(): Promise<MicrosoftLoginResult> {
    return invoke<MicrosoftLoginResult>('refresh_microsoft_auth');
  },

  /** 换皮肤不会触发 token 刷新，需用已存 access_token 重新取地址 */
  async refreshSkinUrl(): Promise<string | null> {
    return invoke<string | null>('refresh_skin_url');
  },

  async logoutMicrosoft(): Promise<void> {
    return invoke('logout_microsoft');
  },

  async setAuthType(authType: string): Promise<void> {
    return invoke('set_auth_type', { authType });
  },
};

export const modpackApi = {
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

  async getModrinthModpackVersions(
    projectId: string,
    gameVersions?: string[],
    loaders?: string[],
  ): Promise<ModrinthVersion[]> {
    return dedupedInvoke<ModrinthVersion[]>('get_modrinth_modpack_versions', {
      projectId,
      gameVersions,
      loaders,
    });
  },

  async installModrinthModpack(options: {
    modpack_id: string;
    version_id: string;
    instance_name: string;
    install_path: string;
  }): Promise<void> {
    return invoke('install_modrinth_modpack', { options });
  },

  async cancelModpackInstall(): Promise<void> {
    return invoke('cancel_modpack_install');
  },
};

export const api = {
  version: versionApi,
  instance: instanceApi,
  loader: loaderApi,
  java: javaApi,
  config: configApi,
  user: userApi,
  modpack: modpackApi,
};

export default api;
