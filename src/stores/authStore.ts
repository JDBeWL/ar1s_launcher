import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { userApi } from '../services'
import { logError } from '../utils/logger'
import type { AuthStatus } from '../types/events'

export const useAuthStore = defineStore('auth', () => {
  const authType = ref<'offline' | 'microsoft'>('offline')
  const username = ref('')
  const msLoggedIn = ref(false)
  const msUsername = ref('')
  const msUuid = ref('')
  const msAccessToken = ref('')
  const msExpiresAt = ref<number | null>(null)

  const isLoggedIn = computed(() => {
    if (authType.value === 'microsoft') return msLoggedIn.value
    return !!(username.value && username.value.trim())
  })

  const displayName = computed(() => {
    if (authType.value === 'microsoft' && msLoggedIn.value) return msUsername.value
    if (authType.value === 'offline' && username.value) return username.value
    return ''
  })

  /** 检查 Microsoft token 是否已过期 */
  const isTokenExpired = computed(() => {
    if (!msExpiresAt.value) return true
    // 提前 60 秒视为过期，避免边界情况
    return Date.now() / 1000 > msExpiresAt.value - 60
  })

  async function loadAuthStatus() {
    try {
      const status: AuthStatus = await userApi.getAuthStatus()
      authType.value = status.authType
      if (status.authType === 'microsoft' && status.loggedIn) {
        msLoggedIn.value = true
        msUsername.value = status.username || ''
        msUuid.value = status.uuid || ''
        msAccessToken.value = status.accessToken || ''
        msExpiresAt.value = status.expiresAt || null
        // 不再启动时自动刷新 token，避免网络不好时导致掉线
        // 用户可通过手动刷新按钮来更新认证信息
      }
    } catch (err) {
      logError('Failed to load auth status', err, 'AuthStore')
    }
  }

  async function tryRefreshMicrosoftToken() {
    try {
      const result = await userApi.refreshMicrosoftAuth()
      msAccessToken.value = result.accessToken
      msUsername.value = result.username
      msUuid.value = result.uuid
      msExpiresAt.value = result.expiresAt
      msLoggedIn.value = true
    } catch {
      // 刷新失败时不清除已有的登录状态，继续使用缓存的认证信息
      // 只有在从未登录过的情况下才标记为未登录
      if (!msAccessToken.value) {
        msLoggedIn.value = false
      }
    }
  }

  async function loadUsername() {
    try {
      const savedUsername = await userApi.getSavedUsername()
      if (savedUsername && typeof savedUsername === 'string') {
        username.value = savedUsername
      }
    } catch (err) {
      logError('Failed to load username', err, 'AuthStore')
    }
  }

  async function saveUsername(newName: string) {
    try {
      await userApi.setSavedUsername(newName)
    } catch (err) {
      logError('Failed to save username', err, 'AuthStore')
    }
  }

  async function switchAuthType(type: 'offline' | 'microsoft') {
    authType.value = type
    try {
      await userApi.setAuthType(type)
    } catch (err) {
      logError('Failed to set auth type', err, 'AuthStore')
    }
  }

  async function requestDeviceCode() {
    const deviceCodeInfo = await userApi.startMicrosoftLogin()
    return deviceCodeInfo
  }

  async function completeMicrosoftLogin(deviceCode: string) {
    const result = await userApi.completeMicrosoftLogin(deviceCode)
    msLoggedIn.value = true
    msUsername.value = result.username
    msUuid.value = result.uuid
    msAccessToken.value = result.accessToken
    return result
  }

  async function logoutMicrosoft() {
    try {
      await userApi.logoutMicrosoft()
      msLoggedIn.value = false
      msUsername.value = ''
      msUuid.value = ''
      msAccessToken.value = ''
      msExpiresAt.value = null
    } catch (err) {
      logError('Microsoft logout failed', err, 'AuthStore')
    }
  }

  async function init() {
    await Promise.all([loadAuthStatus(), loadUsername()])
  }

  return {
    authType,
    username,
    msLoggedIn,
    msUsername,
    msUuid,
    msAccessToken,
    msExpiresAt,
    isLoggedIn,
    isTokenExpired,
    displayName,
    loadAuthStatus,
    tryRefreshMicrosoftToken,
    loadUsername,
    saveUsername,
    switchAuthType,
    requestDeviceCode,
    completeMicrosoftLogin,
    logoutMicrosoft,
    init,
  }
})
