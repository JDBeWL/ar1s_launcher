import { defineStore } from 'pinia'
import { ref } from 'vue'

export type NotificationType = 'success' | 'error' | 'warning' | 'info'

export interface Notification {
  id: number
  type: NotificationType
  title: string
  message?: string
  timeout?: number
  showDetail?: boolean
}

let notificationId = 0

export const useNotificationStore = defineStore('notification', () => {
  // Snackbar 通知队列
  const notifications = ref<Notification[]>([])
  
  // Dialog 详情
  const dialogVisible = ref(false)
  const dialogTitle = ref('')
  const dialogContent = ref('')
  const dialogType = ref<NotificationType>('info')

  // 确认对话框
  const confirmVisible = ref(false)
  const confirmTitle = ref('')
  const confirmContent = ref('')
  const confirmType = ref<NotificationType>('warning')
  let confirmResolve: ((value: boolean) => void) | null = null

  function notify(type: NotificationType, title: string, message?: string, timeout = 4000) {
    const id = ++notificationId
    notifications.value.push({ id, type, title, message, timeout })
    
    if (timeout > 0) {
      setTimeout(() => {
        removeNotification(id)
      }, timeout)
    }
    
    return id
  }

  function success(title: string, message?: string) {
    return notify('success', title, message)
  }

  function error(title: string, message?: string, showDialog = false) {
    if (showDialog && message) {
      showErrorDialog(title, message)
    }
    return notify('error', title, message, 5000)
  }

  function warning(title: string, message?: string) {
    return notify('warning', title, message)
  }

  function info(title: string, message?: string) {
    return notify('info', title, message)
  }

  function removeNotification(id: number) {
    const index = notifications.value.findIndex(n => n.id === id)
    if (index > -1) {
      notifications.value.splice(index, 1)
    }
  }

  function showErrorDialog(title: string, content: string) {
    dialogTitle.value = title
    dialogContent.value = content
    dialogType.value = 'error'
    dialogVisible.value = true
  }

  function showInfoDialog(title: string, content: string) {
    dialogTitle.value = title
    dialogContent.value = content
    dialogType.value = 'info'
    dialogVisible.value = true
  }

  function closeDialog() {
    dialogVisible.value = false
  }

  // 确认对话框
  function confirm(title: string, content: string, type: NotificationType = 'warning'): Promise<boolean> {
    confirmTitle.value = title
    confirmContent.value = content
    confirmType.value = type
    confirmVisible.value = true
    
    return new Promise((resolve) => {
      confirmResolve = resolve
    })
  }

  function handleConfirm(result: boolean) {
    confirmVisible.value = false
    if (confirmResolve) {
      confirmResolve(result)
      confirmResolve = null
    }
  }

  return {
    notifications,
    dialogVisible,
    dialogTitle,
    dialogContent,
    dialogType,
    confirmVisible,
    confirmTitle,
    confirmContent,
    confirmType,
    notify,
    success,
    error,
    warning,
    info,
    removeNotification,
    showErrorDialog,
    showInfoDialog,
    closeDialog,
    confirm,
    handleConfirm
  }
})
