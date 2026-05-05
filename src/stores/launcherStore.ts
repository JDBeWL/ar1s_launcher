import { defineStore } from 'pinia'
import { useMultiEventSubscription } from '../composables/useEventSubscription'
import { useNotificationStore } from './notificationStore'
import { logError } from '../utils/logger'

export const useLauncherStore = defineStore('launcher', () => {
  const eventSubs = useMultiEventSubscription()

  eventSubs.add<string>('minecraft-launched', (event) => {
    const msg = String(event.payload ?? '游戏已启动')
    const notificationStore = useNotificationStore()
    notificationStore.success('Minecraft 已启动', msg)
  })

  eventSubs.add<string>('minecraft-exited', (event) => {
    const msg = String(event.payload ?? '游戏已退出')
    const notificationStore = useNotificationStore()
    notificationStore.info('Minecraft 已退出', msg)
  })

  eventSubs.add<string>('minecraft-error', (event) => {
    const msg = String(event.payload ?? '未知错误')
    logError('Minecraft 运行错误', msg, 'LauncherStore')
    const notificationStore = useNotificationStore()
    notificationStore.error('Minecraft 运行错误', msg, true)
  })

  async function subscribe() {
    await eventSubs.subscribeAll()
  }

  function unsubscribe() {
    eventSubs.unsubscribeAll()
  }

  return {
    subscribe,
    unsubscribe,
  }
})
