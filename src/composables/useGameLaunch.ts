import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../stores/settings';
import { listen } from '@tauri-apps/api/event';

export interface DownloadProgress {
    progress: number;
    total: number;
    speed: number;
    status: 'downloading' | 'completed' | 'cancelled' | 'error';
    percent: number;
    error?: string;
}

export function useGameLaunch() {
    const loading = ref(false);
    const isRepairing = ref(false);
    const repairProgress = ref<DownloadProgress | null>(null);
    const settingsStore = useSettingsStore();

    async function launchGame(
        version: string,
        username: string,
        offline: boolean,
        gameDir: string
    ) {
        if (!version) {
            alert('请先选择一个版本');
            return;
        }

        try {
            loading.value = true;

            // 启动前完整性检查
            const missingFiles = await invoke('validate_version_files', {
                versionId: version
            }) as string[];

            if (missingFiles.length > 0) {
                // 检测到文件缺失，直接修复（不需要用户确认）
                loading.value = false;
                await repairGame(version);
                return;
            }

            // 启动游戏
            await invoke('launch_minecraft', {
                options: {
                    version: version,
                    memory: settingsStore.maxMemory,
                    username: username,
                    offline: offline,
                    game_dir: gameDir
                }
            });
            loading.value = false;
        } catch (err) {
            console.error('Failed to launch game:', err);
            loading.value = false;
            alert(`启动失败: ${err}`);
        }
    }

    async function repairGame(version: string) {
        isRepairing.value = true;
        repairProgress.value = {
            progress: 0,
            total: 0,
            speed: 0,
            status: 'downloading',
            percent: 0
        };

        const unlisten = await listen<DownloadProgress>('download-progress', (event) => {
            repairProgress.value = event.payload;
        });

        try {
            // Map 'bmcl' to the actual URL if needed, or just pass the key if the backend handles it.
            // Looking at download_controller.rs, it takes Option<String> for mirror.
            // Looking at download.rs, apply_mirror takes the mirror URL.
            // The settings store stores 'bmcl' or 'official'.
            // I need to map 'bmcl' to the URL.

            let mirrorUrl: string | null = null;
            if (settingsStore.downloadMirror === 'bmcl') {
                mirrorUrl = 'https://bmclapi2.bangbang93.com';
            }

            await invoke('download_version', {
                versionId: version,
                mirror: mirrorUrl,
            });
            alert('修复完成，请重新启动游戏');
        } catch (err) {
            console.error('Repair failed:', err);
            alert(`修复失败: ${err}`);
        } finally {
            unlisten();
            isRepairing.value = false;
            repairProgress.value = null;
        }
    }

    return {
        loading,
        isRepairing,
        repairProgress,
        launchGame
    };
}
