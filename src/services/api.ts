/**
 * 前端 API 服务层
 * 统一封装 Tauri invoke 调用，提供类型安全和请求去重
 */
import { invoke } from '@tauri-apps/api/core';
import type {
  VersionManifest,
  GameInstance,
  AvailableLoaders,
  ForgeVersion,
  LoaderVersionInfo,
  InstanceNameValidation,
} from '../types/events';

// ============ 请求去重机制 ============

/** 进行中的请求缓存 */
const pendingRequests = new Map<string, Promise<any>>();

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
  /** 获取 Minecraft 版本列表 */
  async getVersions(): Promise<VersionManifest> {
    return dedupedInvoke<VersionManifest>('get_versions');
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
  /** 获取实例列表 */
  async getInstances(): Promise<GameInstance[]> {
    return dedupedInvoke<GameInstance[]>('get_instances');
  },

  /** 创建实例 */
  async createInstance(
    newInstanceName: string,
    baseVersionId: string,
    loader?: { type: string; mc_version: string; loader_version: string }
  ): Promise<void> {
    return invoke('create_instance', { newInstanceName, baseVersionId, loader });
  },

  /** 删除实例 */
  async deleteInstance(instanceName: string): Promise<void> {
    return invoke('delete_instance', { instanceName });
  },

  /** 重命名实例 */
  async renameInstance(oldName: string, newName: string): Promise<void> {
    return invoke('rename_instance', { oldName, newName });
  },

  /** 打开实例文件夹 */
  async openInstanceFolder(instanceName: string): Promise<void> {
    return invoke('open_instance_folder', { instanceName });
  },

  /** 启动实例 */
  async launchInstance(instanceName: string): Promise<void> {
    return invoke('launch_instance', { instanceName });
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
  /** 获取可用的加载器类型 */
  async getAvailableLoaders(minecraftVersion: string): Promise<AvailableLoaders> {
    return dedupedInvoke<AvailableLoaders>('get_available_loaders', { minecraftVersion });
  },

  /** 获取 Forge 版本列表 */
  async getForgeVersions(minecraftVersion: string): Promise<ForgeVersion[]> {
    return dedupedInvoke<ForgeVersion[]>('get_forge_versions', { minecraftVersion });
  },

  /** 获取 Fabric 版本列表 */
  async getFabricVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return dedupedInvoke<LoaderVersionInfo[]>('get_fabric_versions', { minecraftVersion });
  },

  /** 获取 Quilt 版本列表 */
  async getQuiltVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return dedupedInvoke<LoaderVersionInfo[]>('get_quilt_versions', { minecraftVersion });
  },

  /** 获取 NeoForge 版本列表 */
  async getNeoForgeVersions(minecraftVersion: string): Promise<LoaderVersionInfo[]> {
    return dedupedInvoke<LoaderVersionInfo[]>('get_neoforge_versions', { minecraftVersion });
  },
};

// ============ Java 相关 API ============

export const javaApi = {
  /** 查找 Java 安装 */
  async findJavaInstallations(): Promise<string[]> {
    return dedupedInvoke<string[]>('find_java_installations_command');
  },

  /** 强制刷新 Java 安装列表 */
  async refreshJavaInstallations(): Promise<string[]> {
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
};

// ============ 配置相关 API ============

export const configApi = {
  /** 获取游戏目录 */
  async getGameDir(): Promise<string> {
    return dedupedInvoke<string>('get_game_dir');
  },

  /** 设置游戏目录 */
  async setGameDir(path: string): Promise<void> {
    return invoke('set_game_dir', { path, window: {} });
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
    return invoke('save_config_key', { key, value });
  },

  /** 获取上次选择的版本 */
  async getLastSelectedVersion(): Promise<string | null> {
    return dedupedInvoke<string | null>('get_last_selected_version');
  },

  /** 设置上次选择的版本 */
  async setLastSelectedVersion(version: string): Promise<void> {
    return invoke('set_last_selected_version', { version });
  },

  /** 获取总内存 */
  async getTotalMemory(): Promise<number> {
    return dedupedInvoke<number>('get_total_memory');
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
};

// ============ 启动器相关 API ============

export const launcherApi = {
  /** 启动 Minecraft */
  async launchMinecraft(options: {
    version: string;
    memory: number;
    username: string;
    offline: boolean;
    game_dir: string;
  }): Promise<void> {
    return invoke('launch_minecraft', { options });
  },
};

// ============ 整合包相关 API ============

export interface ModrinthSearchResult {
  hits: Array<{
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
  }>;
  total_hits: number;
}

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
  ): Promise<any[]> {
    return dedupedInvoke<any[]>('get_modrinth_modpack_versions', {
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
