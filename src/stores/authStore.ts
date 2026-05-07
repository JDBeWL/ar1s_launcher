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

  const isTokenExpired = computed(() => {
    if (!msExpiresAt.value) return true
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
        msExpiresAt.value = status.expiresAt || null
      }
    } catch (err) {
      logError('Failed to load auth status', err, 'AuthStore')
    }
  }

  async function tryRefreshMicrosoftToken() {
    try {
      const result = await userApi.refreshMicrosoftAuth()
      msUsername.value = result.username
      msUuid.value = result.uuid
      msExpiresAt.value = result.expiresAt
      msLoggedIn.value = true
    } catch {
      if (!msLoggedIn.value) {
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
    const display = await userApi.startMicrosoftLogin()
    return display
  }

  async function completeMicrosoftLogin() {
    const result = await userApi.completeMicrosoftLogin()
    msLoggedIn.value = true
    msUsername.value = result.username
    msUuid.value = result.uuid
    msExpiresAt.value = result.expiresAt
    return result
  }

  async function startAuthCodeLogin() {
    const authUrl = await userApi.startMicrosoftAuthCodeLogin()
    return authUrl
  }

  async function completeAuthCodeLogin() {
    const result = await userApi.completeMicrosoftAuthCodeLogin()
    msLoggedIn.value = true
    msUsername.value = result.username
    msUuid.value = result.uuid
    msExpiresAt.value = result.expiresAt
    return result
  }

  async function logoutMicrosoft() {
    try {
      await userApi.logoutMicrosoft()
      msLoggedIn.value = false
      msUsername.value = ''
      msUuid.value = ''
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
    startAuthCodeLogin,
    completeAuthCodeLogin,
    logoutMicrosoft,
    init,
  }
})
