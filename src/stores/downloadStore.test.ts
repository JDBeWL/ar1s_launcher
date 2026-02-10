import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useDownloadStore } from './downloadStore'
import * as api from '../services/api'

// Mock API
vi.mock('../services/api', () => ({
  api: {
    version: {
      downloadVersion: vi.fn(),
      cancelDownload: vi.fn(),
    },
  },
}))

// Mock notification store
vi.mock('./notificationStore', () => ({
  useNotificationStore: () => ({
    error: vi.fn(),
  }),
}))

// Mock event listener
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
}))

describe('DownloadStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('初始状态正确', () => {
    const store = useDownloadStore()
    
    expect(store.selectedVersion).toBe('')
    expect(store.downloadProgress.status).toBe('idle')
    expect(store.isDownloading).toBe(false)
    expect(store.showNotification).toBe(false)
  })

  it('startDownload 设置状态并调用 API', async () => {
    const store = useDownloadStore()
    const mockDownload = vi.mocked(api.api.version.downloadVersion).mockResolvedValue(undefined)

    await store.startDownload('1.20.1', 'bmcl')

    expect(store.selectedVersion).toBe('1.20.1')
    expect(store.downloadProgress.status).toBe('downloading')
    expect(store.downloadError).toBeNull()
    expect(mockDownload).toHaveBeenCalledWith('1.20.1', 'bmcl')
  })

  it('startDownload 失败时设置错误状态', async () => {
    const store = useDownloadStore()
    const error = new Error('Download failed')
    vi.mocked(api.api.version.downloadVersion).mockRejectedValue(error)

    await store.startDownload('1.20.1')

    expect(store.downloadProgress.status).toBe('error')
    expect(store.downloadError).toBeTruthy()
  })

  it('cancelDownload 调用 API', async () => {
    const store = useDownloadStore()
    const mockCancel = vi.mocked(api.api.version.cancelDownload).mockResolvedValue(undefined)

    await store.cancelDownload()

    expect(mockCancel).toHaveBeenCalled()
  })

  it('toggleNotification 切换通知显示', () => {
    const store = useDownloadStore()
    
    expect(store.showNotification).toBe(false)
    
    store.toggleNotification()
    expect(store.showNotification).toBe(true)
    
    store.toggleNotification()
    expect(store.showNotification).toBe(false)
  })

  it('hideDownloadNotification 隐藏并标记用户已隐藏', () => {
    const store = useDownloadStore()
    
    store.hideDownloadNotification()
    
    expect(store.showNotification).toBe(false)
    expect(store.userHidNotification).toBe(true)
  })
})

