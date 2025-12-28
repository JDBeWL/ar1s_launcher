import { ref, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../stores/settings';
import { useNotificationStore } from '../stores/notificationStore';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { DownloadProgress } from '../types/events';

export function useGameLaunch() {
    const loading = ref(false);
    const isRepairing = ref(false);
    const repairProgress = ref<DownloadProgress | null>(null);
    const settingsStore = useSettingsStore();
    const notificationStore = useNotificationStore();
    
    let unlistenRepairProgress: UnlistenFn | null = null;

    async function launchGame(
        version: string,
        username: string,
        offline: boolean,
        gameDir: string
    ) {
        if (!version) {
            notificationStore.warning('请先选择一个版本');
            return;
        }

        try {
            loading.value = true;

            const missingFiles = await invoke<string[]>('validate_version_files', {
                versionId: version
            });

            if (missingFiles.length > 0) {
                loading.value = false;
                
                // 询问用户是否修复
                const shouldRepair = await notificationStore.confirm(
                    '文件缺失',
                    `检测到 ${missingFiles.length} 个游戏文件缺失，是否立即修复？`,
                    'warning'
                );
                
                if (shouldRepair) {
                    await repairGame(version);
                }
                return;
            }

            await invoke('launch_minecraft', {
                options: {
                    version: version,
                    memory: settingsStore.maxMemory,
                    username: username,
                    offline: offline,
                    game_dir: gameDir
                }
            });
        } catch (err) {
            console.error('Failed to launch game:', err);
            const errorMessage = err instanceof Error ? err.message : String(err);
            notificationStore.error('启动失败', errorMessage, true);
        } finally {
            loading.value = false;
        }
    }

    async function repairGame(version: string) {
        isRepairing.value = true;
        repairProgress.value = {
            progress: 0,
            total: 0,
            speed: 0,
            status: 'downloading',
            bytes_downloaded: 0,
            total_bytes: 0,
            percent: 0
        };

        // 清理之前的监听器
        cleanupRepairListener();
        
        unlistenRepairProgress = await listen<DownloadProgress>('download-progress', (event) => {
            repairProgress.value = event.payload;
        });

        try {
            let mirrorUrl: string | null = null;
            if (settingsStore.downloadMirror === 'bmcl') {
                mirrorUrl = 'https://bmclapi2.bangbang93.com';
            }

            await invoke('download_version', {
                versionId: version,
                mirror: mirrorUrl,
            });
            notificationStore.success('修复完成', '请重新启动游戏');
        } catch (err) {
            console.error('Repair failed:', err);
            const errorMessage = err instanceof Error ? err.message : String(err);
            notificationStore.error('修复失败', errorMessage, true);
        } finally {
            cleanupRepairListener();
            isRepairing.value = false;
            repairProgress.value = null;
        }
    }

    function cleanupRepairListener() {
        if (unlistenRepairProgress) {
            unlistenRepairProgress();
            unlistenRepairProgress = null;
        }
    }

    function cleanup() {
        cleanupRepairListener();
    }

    // 组件卸载时自动清理
    onUnmounted(cleanup);

    return {
        loading,
        isRepairing,
        repairProgress,
        launchGame,
        cleanup
    };
}
