/**
 * 已安装版本管理
 * 读取游戏目录里的已安装版本，未选中时优先恢复上次选择
 */
import { ref } from 'vue';
import { useEventSubscription } from './useEventSubscription';
import { configApi } from '../services';
import { logError } from '../utils/logger';

export function useVersionManager() {
  const installedVersions = ref<string[]>([]);
  const selectedVersion = ref('');
  const loading = ref(false);
  const gameDir = ref('');

  const eventSub = useEventSubscription<string>('game-dir-changed', (event) => {
    gameDir.value = event.payload;
    loadInstalledVersions();
  });

  async function loadGameDir() {
    try {
      gameDir.value = await configApi.getGameDir();
      await loadInstalledVersions();
    } catch (err) {
      logError('Failed to get game directory', err, 'useVersionManager');
    }
  }

  async function loadInstalledVersions() {
    try {
      loading.value = true;
      const dirInfo = await configApi.getGameDirInfo();
      if (dirInfo?.versions) {
        installedVersions.value = dirInfo.versions;

        if (!selectedVersion.value) {
          const lastVersion = await configApi.getLastSelectedVersion();
          if (lastVersion && installedVersions.value.includes(lastVersion)) {
            selectedVersion.value = lastVersion;
          } else if (installedVersions.value.length > 0) {
            selectedVersion.value = installedVersions.value[0];
          }
        }
      }
    } catch (err) {
      logError('Failed to get installed versions', err, 'useVersionManager');
    } finally {
      loading.value = false;
    }
  }

  async function initListeners() {
    await eventSub.subscribe();
    // 另一个实例可能刚创建/删除了版本；切回窗口时重新扫一遍目录
    window.addEventListener('focus', loadGameDir);
  }

  function cleanup() {
    eventSub.unsubscribe();
    window.removeEventListener('focus', loadGameDir);
  }

  return {
    installedVersions,
    selectedVersion,
    loading,
    gameDir,
    loadGameDir,
    loadInstalledVersions,
    initListeners,
    cleanup,
  };
}
