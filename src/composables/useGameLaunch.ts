/**
 * 游戏启动流程
 * 唯一启动入口：登录态与 Java 兼容性前置校验，文件缺失可触发修复，最终交给 launch_instance
 */
import { ref } from 'vue';
import { useEventSubscription } from './useEventSubscription';
import { versionApi, instanceApi, javaApi } from '../services';
import { useSettingsStore } from '../stores/settings';
import { useNotificationStore } from '../stores/notificationStore';
import { useAuthStore } from '../stores/authStore';
import { getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import type { DownloadProgress, JavaCompatibilityResult } from '../types/events';

interface JavaResolveResult {
  shouldLaunch: boolean;
  overrideJavaPath?: string;
  forceJavaMismatch?: boolean;
}

export function useGameLaunch() {
  const loading = ref(false);
  const isRepairing = ref(false);
  const repairProgress = ref<DownloadProgress | null>(null);
  const settingsStore = useSettingsStore();
  const notificationStore = useNotificationStore();
  const authStore = useAuthStore();

  const repairEventSub = useEventSubscription<DownloadProgress>('download-progress', (event) => {
    if (isRepairing.value) {
      repairProgress.value = event.payload;
    }
  });

  async function checkAndResolveJava(instanceName: string): Promise<JavaResolveResult> {
    let result: JavaCompatibilityResult;
    try {
      result = await javaApi.checkJavaCompatibility(instanceName);
    } catch {
      return { shouldLaunch: true };
    }

    // 能自动换到兼容 Java 的话交给后端换，不用打扰用户；换不到就必须让人确认
    if (result.compatible || (result.autoMatchEnabled && result.recommendedJavaPath)) {
      return { shouldLaunch: true };
    }

    const currentVer = result.currentJavaVersion != null ? `Java ${result.currentJavaVersion}` : '未知';
    const requiredVer = `Java ${result.requiredJavaVersion}`;

    const content = result.hardBlock
      ? `当前 Java 起不了这个实例，后端给出的诊断如下：\n\n${result.diagnostic}\n\n请选择如何处理：`
      : `当前 Java 版本 (${currentVer}) 不满足 Minecraft ${instanceName} 的要求 (需要 ${requiredVer})。\n\n请选择如何处理：`;

    const options: Array<{
      id: string;
      label: string;
      color?: string;
      variant?: 'elevated' | 'outlined' | 'text' | 'flat' | 'tonal' | 'plain';
    }> = [];

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

    // 判据没覆盖全部情形，所以坚持要起时起得来；hardBlock 时这一条就是带风险的确认
    options.push({
      id: 'continue',
      label: result.recommendedJavaPath ? '继续启动' : '仍用当前 Java 启动（风险自负）',
      color: result.recommendedJavaPath ? 'info' : 'error',
      variant: 'text',
    });

    const selected = await notificationStore.choice('Java 版本提示', content, options, 'warning');

    if (selected === 'continue') {
      return { shouldLaunch: true, forceJavaMismatch: result.hardBlock };
    }

    if (selected === 'temp' && result.recommendedJavaPath) {
      return { shouldLaunch: true, overrideJavaPath: result.recommendedJavaPath };
    }

    if (selected === 'auto') {
      settingsStore.autoMatchJava = true;
      await settingsStore.saveAutoMatchJava();
    } else if (selected === 'manual') {
      notificationStore.warning('请在设置中手动选择兼容的 Java 路径');
    }

    return { shouldLaunch: false };
  }

  async function repairInstance(instanceName: string) {
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

      await versionApi.downloadVersion(instanceName, mirror);
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

  // 唯一启动入口；认证与内存设置由后端 launch_instance 从配置读取，这里只做前置校验
  async function launchInstance(instanceName: string): Promise<boolean> {
    if (!instanceName) {
      notificationStore.warning('请先选择一个实例');
      return false;
    }

    if (!authStore.isLoggedIn) {
      notificationStore.warning('请先设置玩家名称或登录 Microsoft 账户');
      return false;
    }

    try {
      loading.value = true;

      if (authStore.authType === 'microsoft' && authStore.msLoggedIn && authStore.isTokenExpired) {
        await authStore.tryRefreshMicrosoftToken();
      }

      const javaResolve = await checkAndResolveJava(instanceName);
      if (!javaResolve.shouldLaunch) {
        return false;
      }

      const missingFiles = await versionApi.validateVersionFiles(instanceName);
      if (missingFiles.length > 0) {
        const shouldRepair = await notificationStore.confirm(
          '文件缺失',
          `检测到 ${missingFiles.length} 个游戏文件缺失，是否立即修复？`,
          'warning',
        );

        if (shouldRepair) {
          await repairInstance(instanceName);
        }
        return false;
      }

      await instanceApi.launchInstance(instanceName, javaResolve.overrideJavaPath, javaResolve.forceJavaMismatch);
      notificationStore.success('启动成功', `${instanceName} 正在启动`);
      return true;
    } catch (err) {
      logError('Failed to launch instance', err, 'useGameLaunch');
      notificationStore.error('启动失败', getErrorMessage(err), true);
      return false;
    } finally {
      loading.value = false;
    }
  }

  return {
    loading,
    isRepairing,
    repairProgress,
    launchInstance,
  };
}
