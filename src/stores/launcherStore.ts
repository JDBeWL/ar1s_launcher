import { defineStore } from 'pinia'
import { onScopeDispose } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useNotificationStore } from './notificationStore'

export const useLauncherStore = defineStore('launcher', () => {
  // Listeners
  let unlistenLaunched: UnlistenFn | null = null;
  let unlistenExited: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  // 当 store 的作用域销毁时自动清理监听器
  onScopeDispose(() => {
    unsubscribe();
  });

  async function subscribe() {
    if (unlistenLaunched) return; // Already subscribed

    const notificationStore = useNotificationStore()

    unlistenLaunched = await listen('minecraft-launched', (event) => {
      const msg = String(event.payload ?? '游戏已启动')
      notificationStore.success('Minecraft 已启动', msg)
    })

    unlistenExited = await listen('minecraft-exited', (event) => {
      const msg = String(event.payload ?? '游戏已退出')
      notificationStore.info('Minecraft 已退出', msg)
    })

    unlistenError = await listen('minecraft-error', (event) => {
      const msg = String(event.payload ?? '未知错误')
      console.error('Minecraft 运行错误:', msg)
      notificationStore.error('Minecraft 运行错误', msg, true)
    })
  }

  function unsubscribe() {
    if (unlistenLaunched) {
      unlistenLaunched();
      unlistenLaunched = null;
    }
    if (unlistenExited) {
      unlistenExited();
      unlistenExited = null;
    }
    if (unlistenError) {
      unlistenError();
      unlistenError = null;
    }
  }

  return {
    subscribe,
    unsubscribe,
  }
})
