import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useVersionManager() {
    const installedVersions = ref<string[]>([]);
    const selectedVersion = ref('');
    const loading = ref(false);
    const gameDir = ref('');

    // 加载已保存的游戏目录
    async function loadGameDir() {
        try {
            const dir = await invoke('get_game_dir');
            gameDir.value = dir as string;
            await loadInstalledVersions();
        } catch (err) {
            console.error('Failed to get game directory:', err);
        }
    }

    // 获取已安装的版本
    async function loadInstalledVersions() {
        try {
            loading.value = true;
            const dirInfo = await invoke('get_game_dir_info');
            if (dirInfo && (dirInfo as any).versions) {
                installedVersions.value = (dirInfo as any).versions;
                if (installedVersions.value.length > 0 && !selectedVersion.value) {
                    selectedVersion.value = installedVersions.value[0];
                }
            }
            loading.value = false;
        } catch (err) {
            console.error('Failed to get installed versions:', err);
            loading.value = false;
        }
    }

    // 初始化监听器
    async function initListeners() {
        await listen('game-dir-changed', (event) => {
            gameDir.value = event.payload as string;
            loadInstalledVersions();
        });
    }

    return {
        installedVersions,
        selectedVersion,
        loading,
        gameDir,
        loadGameDir,
        loadInstalledVersions,
        initListeners
    };
}
