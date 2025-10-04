import { defineStore } from 'pinia'
import { ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'

export const useLauncherStore = defineStore('launcher', () => {
  // State
  const gameSnackVisible = ref(false)
  const gameSnackText = ref('')
  const gameSnackColor = ref<'info' | 'success' | 'warning' | 'error'>('info')

  const gameDialogVisible = ref(false)
  const gameDialogTitle = ref('')
  const gameDialogText = ref('')

  // Listeners
  let unlistenLaunched: UnlistenFn | null = null;
  let unlistenExited: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function subscribe() {
    if (unlistenLaunched) return; // Already subscribed

    unlistenLaunched = await listen('minecraft-launched', (event) => {
      const msg = String(event.payload ?? '游戏已启动')
      gameSnackText.value = `Minecraft 已启动：${msg}`
      gameSnackColor.value = 'success'
      gameSnackVisible.value = true
    })

    unlistenExited = await listen('minecraft-exited', (event) => {
      const msg = String(event.payload ?? '游戏已退出')
      gameSnackText.value = `Minecraft 已退出：${msg}`
      gameSnackColor.value = 'info'
      gameSnackVisible.value = true
    })

    unlistenError = await listen('minecraft-error', (event) => {
      const msg = String(event.payload ?? '未知错误')
      console.error('Minecraft 运行错误:', msg)
      gameDialogTitle.value = 'Minecraft 运行错误'
      gameDialogText.value = msg
      gameDialogVisible.value = true
      gameSnackText.value = 'Minecraft 发生错误'
      gameSnackColor.value = 'error'
      gameSnackVisible.value = true
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
    gameSnackVisible,
    gameSnackText,
    gameSnackColor,
    gameDialogVisible,
    gameDialogTitle,
    gameDialogText,
    subscribe,
    unsubscribe,
  }
})