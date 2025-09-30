import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, emit } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

export const useDownloadStore = defineStore('download', () => {
  // 下载状态
  const selectedVersion = ref('')
  const downloadProgress = ref({
    progress: 0,
    total: 0,
    speed: 0,
    status: 'idle' // idle, downloading, completed, cancelled, error
  })
  const isDownloading = computed(() => downloadProgress.value.status === 'downloading')
  const completionNotified = ref(false)
  
  // 控制通知显示状态
  const showNotification = ref(false)
  // 用户是否手动隐藏了通知
  const userHidNotification = ref(false)

  // 初始化监听器
  async function initListeners() {
    await listen('download-progress', (event) => {
      const data = event.payload as any
      downloadProgress.value = data

      if (data.status === 'downloading' && !userHidNotification.value) {
        // 当开始下载时自动显示通知，除非用户手动隐藏了
        showNotification.value = true
      }

      if (data.status === 'completed' && !completionNotified.value) {
        completionNotified.value = true
        // 下载完成时保持通知显示，除非用户手动隐藏了
        if (!userHidNotification.value) {
          showNotification.value = true
        }
        // 这里可以添加全局通知逻辑
      } else if (data.status === 'cancelled' || data.status === 'error') {
        completionNotified.value = false
        selectedVersion.value = ''
        // 重置用户隐藏标志
        userHidNotification.value = false
      }
    })
  }

  // 开始下载
  async function startDownload(versionId: string, source: string = 'bmcl') {
    selectedVersion.value = versionId
    downloadProgress.value.status = 'downloading'
    completionNotified.value = false
    userHidNotification.value = false // 重置用户隐藏标志
    showNotification.value = true // 开始下载时显示通知
    
    try {
      await invoke('download_version', { 
        versionId: selectedVersion.value,
        mirror: source === 'bmcl' ? 'bmcl' : undefined,
      })
    } catch (err) {
      console.error('Failed to download version:', err)
      downloadProgress.value.status = 'error'
      const errorMessage = (err as any).message || String(err)
      alert(`下载失败: ${errorMessage}`)
    }
  }

  // 取消下载
  async function cancelDownload() {
    await emit('cancel-download')
  }

  // 切换通知显示状态
  function toggleNotification() {
    showNotification.value = !showNotification.value
  }

  // 显示通知
  function showDownloadNotification() {
    showNotification.value = true
  }

  // 隐藏通知
  function hideDownloadNotification() {
    showNotification.value = false
    userHidNotification.value = true // 标记用户手动隐藏了通知
  }

  return {
    selectedVersion,
    downloadProgress,
    isDownloading,
    completionNotified,
    showNotification,
    userHidNotification,
    initListeners,
    startDownload,
    cancelDownload,
    toggleNotification,
    showDownloadNotification,
    hideDownloadNotification
  }
})