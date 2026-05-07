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

export interface ChoiceOption {
  id: string
  label: string
  description?: string
  color?: string
  variant?: 'elevated' | 'outlined' | 'text' | 'flat' | 'tonal' | 'plain'
}

let notificationId = 0
const MAX_NOTIFICATIONS = 5

export const useNotificationStore = defineStore('notification', () => {
  const notifications = ref<Notification[]>([])
  
  const dialogVisible = ref(false)
  const dialogTitle = ref('')
  const dialogContent = ref('')
  const dialogType = ref<NotificationType>('info')

  const confirmVisible = ref(false)
  const confirmTitle = ref('')
  const confirmContent = ref('')
  const confirmType = ref<NotificationType>('warning')
  let confirmResolve: ((value: boolean) => void) | null = null

  const choiceVisible = ref(false)
  const choiceTitle = ref('')
  const choiceContent = ref('')
  const choiceType = ref<NotificationType>('warning')
  const choiceOptions = ref<ChoiceOption[]>([])
  let choiceResolve: ((value: string | null) => void) | null = null

  function notify(type: NotificationType, title: string, message?: string, timeout = 4000) {
    const id = ++notificationId
    notifications.value.push({ id, type, title, message, timeout })
    
    if (notifications.value.length > MAX_NOTIFICATIONS) {
      notifications.value.splice(0, notifications.value.length - MAX_NOTIFICATIONS)
    }
    
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
      // 显示详情对话框时，不再显示 snackbar 通知
      showErrorDialog(title, message)
      return -1
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

  function choice(
    title: string,
    content: string,
    options: ChoiceOption[],
    type: NotificationType = 'warning'
  ): Promise<string | null> {
    choiceTitle.value = title
    choiceContent.value = content
    choiceType.value = type
    choiceOptions.value = options
    choiceVisible.value = true

    return new Promise((resolve) => {
      choiceResolve = resolve
    })
  }

  function handleChoice(result: string | null) {
    choiceVisible.value = false
    if (choiceResolve) {
      choiceResolve(result)
      choiceResolve = null
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
    handleConfirm,
    choiceVisible,
    choiceTitle,
    choiceContent,
    choiceType,
    choiceOptions,
    choice,
    handleChoice
  }
})
