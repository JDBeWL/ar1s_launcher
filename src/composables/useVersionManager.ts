import { ref, onScopeDispose } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { configApi } from '../services';
import { logError } from '../utils/logger';

export function useVersionManager() {
    const installedVersions = ref<string[]>([]);
    const selectedVersion = ref('');
    const loading = ref(false);
    const gameDir = ref('');
    
    let unlistenGameDirChanged: UnlistenFn | null = null;

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
                
                // 尝试加载上次选择的版本
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
        // 避免重复监听
        if (unlistenGameDirChanged) return;
        
        unlistenGameDirChanged = await listen<string>('game-dir-changed', (event) => {
            gameDir.value = event.payload;
            loadInstalledVersions();
        });
    }

    function cleanup() {
        if (unlistenGameDirChanged) {
            unlistenGameDirChanged();
            unlistenGameDirChanged = null;
        }
    }

    // 作用域销毁时自动清理
    onScopeDispose(cleanup);

    return {
        installedVersions,
        selectedVersion,
        loading,
        gameDir,
        loadGameDir,
        loadInstalledVersions,
        initListeners,
        cleanup
    };
}
