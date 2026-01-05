import { ref, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

interface GameDirInfo {
    versions: string[];
}

export function useVersionManager() {
    const installedVersions = ref<string[]>([]);
    const selectedVersion = ref('');
    const loading = ref(false);
    const gameDir = ref('');
    
    let unlistenGameDirChanged: UnlistenFn | null = null;

    async function loadGameDir() {
        try {
            const dir = await invoke<string>('get_game_dir');
            gameDir.value = dir;
            await loadInstalledVersions();
        } catch (err) {
            console.error('Failed to get game directory:', err);
        }
    }

    async function loadInstalledVersions() {
        try {
            loading.value = true;
            const dirInfo = await invoke<GameDirInfo>('get_game_dir_info');
            if (dirInfo?.versions) {
                installedVersions.value = dirInfo.versions;
                
                // 尝试加载上次选择的版本
                if (!selectedVersion.value) {
                    const lastVersion = await invoke<string | null>('get_last_selected_version');
                    if (lastVersion && installedVersions.value.includes(lastVersion)) {
                        selectedVersion.value = lastVersion;
                    } else if (installedVersions.value.length > 0) {
                        selectedVersion.value = installedVersions.value[0];
                    }
                }
            }
        } catch (err) {
            console.error('Failed to get installed versions:', err);
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

    // 组件卸载时自动清理
    onUnmounted(cleanup);

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
