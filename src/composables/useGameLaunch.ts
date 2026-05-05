import { ref } from 'vue';
import { useEventSubscription } from './useEventSubscription';
import { versionApi, launcherApi } from '../services';
import { useSettingsStore } from '../stores/settings';
import { useNotificationStore } from '../stores/notificationStore';
import { getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import type { DownloadProgress } from '../types/events';

interface AuthInfo {
    authType: string;
    accessToken: string;
    uuid: string;
}

export function useGameLaunch() {
    const loading = ref(false);
    const isRepairing = ref(false);
    const repairProgress = ref<DownloadProgress | null>(null);
    const settingsStore = useSettingsStore();
    const notificationStore = useNotificationStore();

    const repairEventSub = useEventSubscription<DownloadProgress>('download-progress', (event) => {
        if (isRepairing.value) {
            repairProgress.value = event.payload;
        }
    });

    async function launchGame(
        version: string,
        username: string,
        auth?: AuthInfo,
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
                auth_type: auth?.authType,
                access_token: auth?.accessToken,
                uuid: auth?.uuid,
            });
        } catch (err) {
            logError('Failed to launch game', err, 'useGameLaunch');
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

        await repairEventSub.subscribe();

        try {
            const mirror = settingsStore.downloadMirror === 'bmcl' ? 'bmcl' : undefined;

            await versionApi.downloadVersion(version, mirror);
            notificationStore.success('修复完成', '请重新启动游戏');
        } catch (err) {
            logError('Repair failed', err, 'useGameLaunch');
            notificationStore.error('修复失败', getErrorMessage(err), true);
        } finally {
            repairEventSub.unsubscribe();
            isRepairing.value = false;
            repairProgress.value = null;
        }
    }

    return {
        loading,
        isRepairing,
        repairProgress,
        launchGame,
    };
}
