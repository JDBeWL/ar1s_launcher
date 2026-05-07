import { defineStore } from 'pinia'
import { ref } from 'vue'
import { configApi, javaApi } from '../services'
import { logError } from '../utils/logger'

export const useSettingsStore = defineStore('settings', () => {
  const maxMemory = ref(4096)
  const totalMemoryMB = ref(0)
  const downloadMirror = ref('bmcl')
  const javaInstallations = ref<string[]>([])
  const hasFoundJavaInstallations = ref(false)
  const autoMatchJava = ref(false)

  async function loadSystemMemory() {
    try {
      totalMemoryMB.value = await configApi.getTotalMemory()
    } catch (err) {
      logError('Failed to get total memory', err, 'SettingsStore')
    }
  }

  async function loadSettings() {
    try {
      const config = await configApi.getConfig()
      if (config.max_memory) {
        maxMemory.value = config.max_memory
      }
      if (config.download_mirror) {
        downloadMirror.value = config.download_mirror
      }
      autoMatchJava.value = config.auto_match_java
    } catch (err) {
      logError('Failed to load settings', err, 'SettingsStore')
    }
  }

  async function saveMaxMemory() {
    try {
      await configApi.saveConfigKey('maxMemory', maxMemory.value.toString())
    } catch (err) {
      logError('Failed to set max memory', err, 'SettingsStore')
    }
  }

  async function saveDownloadMirror() {
    try {
      await configApi.saveConfigKey('downloadMirror', downloadMirror.value)
    } catch (err) {
      logError('Failed to set download mirror', err, 'SettingsStore')
    }
  }

  async function findJavaInstallations() {
    try {
      const installations = await javaApi.findJavaInstallations()
      javaInstallations.value = installations
      hasFoundJavaInstallations.value = true
      return javaInstallations.value
    } catch (err) {
      logError('Failed to find Java installations', err, 'SettingsStore')
      return []
    }
  }

  async function saveAutoMatchJava() {
    try {
      await configApi.saveConfigKey('autoMatchJava', autoMatchJava.value.toString())
    } catch (err) {
      logError('Failed to save auto match java', err, 'SettingsStore')
    }
  }

  return {
    maxMemory,
    totalMemoryMB,
    downloadMirror,
    javaInstallations,
    hasFoundJavaInstallations,
    autoMatchJava,
    loadSystemMemory,
    loadSettings,
    saveMaxMemory,
    saveDownloadMirror,
    findJavaInstallations,
    saveAutoMatchJava
  }
})
