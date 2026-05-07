import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { emit } from '@tauri-apps/api/event'
import type { DownloadProgress, DownloadStatus } from '../types/events'
import { api } from '../services/api'
import { useNotificationStore } from './notificationStore'
import { getErrorMessage } from '../utils/format'
import { logError } from '../utils/logger'

export type StoreDownloadStatus = DownloadStatus | 'idle';

export interface DownloadState extends Omit<DownloadProgress, 'status'> {
  status: StoreDownloadStatus;
}

let downloadProgressUnlisten: (() => void) | null = null;

export const useDownloadStore = defineStore('download', () => {
  const selectedVersion = ref('')
  const downloadError = ref<string | null>(null);
  const downloadProgress = ref<DownloadState>({
    bytes_downloaded: 0,
    total_bytes: 0,
    speed: 0,
    status: 'idle',
    percent: 0,
    error: undefined,
  })
  const isDownloading = computed(() => downloadProgress.value.status === 'downloading')
  const completionNotified = ref(false)
  const showNotification = ref(false)
  const userHidNotification = ref(false)
  const isInitialized = ref(false)

  function handleDownloadProgress(event: DownloadProgress) {
    const data = event as DownloadState;

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
          const notificationStore = useNotificationStore()
          notificationStore.error('下载失败', data.error || '下载过程中发生未知错误', true)
      }
    }
  }

  async function init() {
    if (isInitialized.value) return;
    
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<DownloadProgress>('download-progress', (event) => {
        downloadProgress.value = event.payload as DownloadState;
        handleDownloadProgress(event.payload);
      });
      downloadProgressUnlisten = unlisten;
      isInitialized.value = true;
      logError('Download store initialized', undefined, 'DownloadStore');
    } catch (err) {
      logError('Failed to initialize download store', err, 'DownloadStore');
    }
  }

  function cleanup() {
    if (downloadProgressUnlisten) {
      downloadProgressUnlisten();
      downloadProgressUnlisten = null;
      isInitialized.value = false;
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
      await api.version.downloadVersion(
        selectedVersion.value,
        source === 'bmcl' ? 'bmcl' : undefined,
      )
    } catch (err) {
      logError('Failed to start download invocation', err, 'DownloadStore')
      downloadProgress.value.status = 'error'
      const errorMessage = getErrorMessage(err);
      downloadError.value = errorMessage;
      const notificationStore = useNotificationStore()
      notificationStore.error('下载失败', errorMessage, true)
    }
  }

  async function cancelDownload() {
    try {
      await api.version.cancelDownload()
    } catch (err) {
      logError('Failed to cancel download', err, 'DownloadStore')
      // 回退到 emit 方式
      await emit('cancel-download')
    }
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

    init,
    cleanup,
    startDownload,
    cancelDownload,
    toggleNotification,
    showDownloadNotification,
    hideDownloadNotification
  }
})