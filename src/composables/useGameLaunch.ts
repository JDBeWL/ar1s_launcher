import { ref, onScopeDispose } from 'vue';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { versionApi, launcherApi } from '../services';
import { useSettingsStore } from '../stores/settings';
import { useNotificationStore } from '../stores/notificationStore';
import { getErrorMessage } from '../utils/format';
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
    ) {
        if (!version) {
            notificationStore.warning('请先选择一个版本');
            return;
        }

        try {
            loading.value = true;

            const missingFiles = await versionApi.validateVersionFiles(version);

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

            await launcherApi.launchMinecraft({
                version,
                username,
                memory: settingsStore.maxMemory,
            });
        } catch (err) {
            console.error('Failed to launch game:', err);
            notificationStore.error('启动失败', getErrorMessage(err), true);
        } finally {
            loading.value = false;
        }
    }

    async function repairGame(version: string) {
        isRepairing.value = true;
        repairProgress.value = {
            bytes_downloaded: 0,
            total_bytes: 0,
            speed: 0,
            status: 'downloading',
            percent: 0,
        };

        // 清理之前的监听器
        cleanupRepairListener();
        
        unlistenRepairProgress = await listen<DownloadProgress>('download-progress', (event) => {
            repairProgress.value = event.payload;
        });

        try {
            const mirror = settingsStore.downloadMirror === 'bmcl' ? 'bmcl' : undefined;

            await versionApi.downloadVersion(version, mirror);
            notificationStore.success('修复完成', '请重新启动游戏');
        } catch (err) {
            console.error('Repair failed:', err);
            notificationStore.error('修复失败', getErrorMessage(err), true);
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

    // 作用域销毁时自动清理
    onScopeDispose(cleanupRepairListener);

    return {
        loading,
        isRepairing,
        repairProgress,
        launchGame,
    };
}
