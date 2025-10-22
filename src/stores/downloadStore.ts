import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, emit } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { DownloadProgress, DownloadStatus } from '../types/events'

// Add 'idle' to the possible statuses for the store
export type StoreDownloadStatus = DownloadStatus | 'idle';

export interface DownloadState extends Omit<DownloadProgress, 'status'> {
  status: StoreDownloadStatus;
}

export const useDownloadStore = defineStore('download', () => {
  // State
  const selectedVersion = ref('')
  const downloadError = ref<string | null>(null);
  const downloadProgress = ref<DownloadState>({
    progress: 0,
    total: 0,
    speed: 0,
    status: 'idle',
    bytes_downloaded: 0,
    total_bytes: 0,
    percent: 0,
    error: undefined,
  })
  const isDownloading = computed(() => downloadProgress.value.status === 'downloading')
  const completionNotified = ref(false)
  const showNotification = ref(false)
  const userHidNotification = ref(false)

  // Listeners
  let unlistenDownloadProgress: UnlistenFn | null = null;

  async function subscribe() {
    if (unlistenDownloadProgress) return;
    unlistenDownloadProgress = await listen('download-progress', (event) => {
      const data = event.payload as DownloadProgress
      downloadProgress.value = data as DownloadState;

      if (data.status === 'downloading' && !userHidNotification.value) {
        showNotification.value = true
      }

      if (data.status === 'completed' && !completionNotified.value) {
        completionNotified.value = true
        if (!userHidNotification.value) {
          showNotification.value = true
        }
      } else if (data.status === 'cancelled' || data.status === 'error') {
        completionNotified.value = false
        selectedVersion.value = ''
        userHidNotification.value = false
        if (data.status === 'error') {
            downloadError.value = data.error || '下载过程中发生未知错误';
            // 显示错误提示
            if (data.error) {
                alert(`下载失败: ${data.error}`);
            }
        }
      }
    })
  }

  function unsubscribe() {
    if (unlistenDownloadProgress) {
      unlistenDownloadProgress();
      unlistenDownloadProgress = null;
    }
  }

  // Actions
  async function startDownload(versionId: string, source: string = 'bmcl') {
    selectedVersion.value = versionId
    downloadProgress.value.status = 'downloading'
    downloadError.value = null; // Reset error on new download
    completionNotified.value = false
    userHidNotification.value = false
    showNotification.value = true
    
    try {
      await invoke('download_version', { 
        versionId: selectedVersion.value,
        mirror: source === 'bmcl' ? 'bmcl' : undefined,
      })
    } catch (err) {
      console.error('Failed to start download invocation:', err)
      downloadProgress.value.status = 'error'
      const errorMessage = err instanceof Error ? err.message : String(err);
      downloadError.value = errorMessage;
      alert(`下载失败: ${errorMessage}`)
    }
  }

  async function cancelDownload() {
    await emit('cancel-download')
  }

  function toggleNotification() {
    showNotification.value = !showNotification.value
  }

  function showDownloadNotification() {
    showNotification.value = true
  }

  function hideDownloadNotification() {
    showNotification.value = false
    userHidNotification.value = true
  }

  return {
    selectedVersion,
    downloadProgress,
    isDownloading,
    completionNotified,
    downloadError,
    showNotification,
    userHidNotification,

    subscribe,
    unsubscribe,
    startDownload,
    cancelDownload,
    toggleNotification,
    showDownloadNotification,
    hideDownloadNotification
  }
})