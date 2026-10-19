/**
 * 游戏进程事件存储
 * exited / error 转成通知，minecraft-crash 转成崩溃诊断弹窗；
 * launched 只表示进程已派生，由启动入口提示“正在启动”，不再重复报成功
 */
import { defineStore } from 'pinia';
import { useMultiEventSubscription } from '../composables/useEventSubscription';
import { translate } from '../i18n';
import { useNotificationStore } from './notificationStore';
import { useCrashStore } from './crashStore';
import type { CrashDiagnosis } from '../types/events';
import { logError } from '../utils/logger';

export const useLauncherStore = defineStore('launcher', () => {
  const eventSubs = useMultiEventSubscription();

  eventSubs.add<string>('minecraft-exited', (event) => {
    const msg = String(event.payload ?? translate('game.exitedHint'));
    const notificationStore = useNotificationStore();
    notificationStore.info(translate('game.exitedTitle'), msg);
  });

  eventSubs.add<string>('minecraft-error', (event) => {
    const msg = String(event.payload ?? translate('errors.globalUnknown'));
    logError('Minecraft runtime error', msg, 'LauncherStore');
    const notificationStore = useNotificationStore();
    notificationStore.error(translate('game.errorTitle'), msg, true);
  });

  eventSubs.add<CrashDiagnosis>('minecraft-crash', (event) => {
    const diagnosis = event.payload;
    if (!diagnosis) return;

    logError('Minecraft launch failed', diagnosis.headline, 'LauncherStore');
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
