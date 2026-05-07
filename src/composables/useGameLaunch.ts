import { ref } from 'vue';
import { useEventSubscription } from './useEventSubscription';
import { versionApi, launcherApi, javaApi } from '../services';
import { useSettingsStore } from '../stores/settings';
import { useNotificationStore } from '../stores/notificationStore';
import { getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import type { DownloadProgress, JavaCompatibilityResult } from '../types/events';

interface AuthInfo {
    authType: string;
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

    interface JavaResolveResult {
        path?: string;
        shouldLaunch: boolean;
    }

    async function checkAndResolveJava(version: string): Promise<JavaResolveResult> {
        let result: JavaCompatibilityResult;
        try {
            result = await javaApi.checkJavaCompatibility(version);
        } catch {
            return { shouldLaunch: true };
        }

        if (result.compatible || result.autoMatchEnabled) {
            return { shouldLaunch: true };
        }

        const currentVer = result.currentJavaVersion != null ? `Java ${result.currentJavaVersion}` : '未知';
        const requiredVer = `Java ${result.requiredJavaVersion}`;

        let content = `当前 Java 版本 (${currentVer}) 不满足 Minecraft ${version} 的要求 (需要 ${requiredVer})。\n\n请选择如何处理：`;

        const options: Array<{ id: string; label: string; color?: string; variant?: 'elevated' | 'outlined' | 'text' | 'flat' | 'tonal' | 'plain' }> = [];

        if (result.recommendedJavaPath) {
            options.push({
                id: 'temp',
                label: `临时使用 Java ${result.recommendedJavaVersion}`,
                color: 'primary',
                variant: 'elevated',
            });
        }

        options.push({
            id: 'auto',
            label: '开启自动匹配',
            color: 'success',
            variant: 'tonal',
        });

        options.push({
            id: 'manual',
            label: '手动选择 Java',
            color: 'warning',
            variant: 'outlined',
        });

        options.push({
            id: 'continue',
            label: '继续启动',
            color: 'info',
            variant: 'text',
        });

        const selected = await notificationStore.choice(
            'Java 版本提示',
            content,
            options,
            'warning'
        );

        if (selected === null) {
            return { path: undefined, shouldLaunch: false };
        }

        if (selected === 'continue') {
            return { path: '', shouldLaunch: true };
        }

        if (selected === 'temp' && result.recommendedJavaPath) {
            return { path: result.recommendedJavaPath, shouldLaunch: true };
        }

        if (selected === 'auto') {
            settingsStore.autoMatchJava = true;
            await settingsStore.saveAutoMatchJava();
            return { path: undefined, shouldLaunch: false };
        }

        if (selected === 'manual') {
            notificationStore.warning('请在设置中手动选择兼容的 Java 路径');
            return { path: undefined, shouldLaunch: false };
        }

        return { path: undefined, shouldLaunch: false };
    }

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

            const javaResolve = await checkAndResolveJava(version);

            if (!javaResolve.shouldLaunch) {
                loading.value = false;
                return;
            }

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

            const launchOptions: any = {
                version,
                username,
                memory: settingsStore.maxMemory,
                auth_type: auth?.authType,
                uuid: auth?.uuid,
            };
            // 只有明确指定了覆盖路径时才传递
            if (javaResolve.path) {
                launchOptions.override_java_path = javaResolve.path;
            }

            await launcherApi.launchMinecraft(launchOptions);
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
