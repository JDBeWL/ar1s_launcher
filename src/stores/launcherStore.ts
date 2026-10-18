/**
 * 游戏进程事件存储
 * exited / error 转成通知，minecraft-crash 转成崩溃诊断弹窗；
 * launched 只表示进程已派生，由启动入口提示“正在启动”，不再重复报成功
 */
import { defineStore } from 'pinia';
import { useMultiEventSubscription } from '../composables/useEventSubscription';
import { useNotificationStore } from './notificationStore';
import { useCrashStore } from './crashStore';
import type { CrashDiagnosis } from '../types/events';
import { logError } from '../utils/logger';

export const useLauncherStore = defineStore('launcher', () => {
  const eventSubs = useMultiEventSubscription();

  eventSubs.add<string>('minecraft-exited', (event) => {
    const msg = String(event.payload ?? '游戏已退出');
    const notificationStore = useNotificationStore();
    notificationStore.info('Minecraft 已退出', msg);
  });

  eventSubs.add<string>('minecraft-error', (event) => {
    const msg = String(event.payload ?? '未知错误');
    logError('Minecraft 运行错误', msg, 'LauncherStore');
    const notificationStore = useNotificationStore();
    notificationStore.error('Minecraft 运行错误', msg, true);
  });

  eventSubs.add<CrashDiagnosis>('minecraft-crash', (event) => {
    const diagnosis = event.payload;
    if (!diagnosis) return;

    logError('Minecraft 启动失败', diagnosis.headline, 'LauncherStore');
    useCrashStore().open(diagnosis);
  });

  async function subscribe() {
    await eventSubs.subscribeAll();
  }

  function unsubscribe() {
    eventSubs.unsubscribeAll();
  }

  return {
    subscribe,
    unsubscribe,
  };
});
