import { defineStore } from 'pinia'
import { ref } from 'vue'
import { configApi, javaApi } from '../services'

export const useSettingsStore = defineStore('settings', () => {
  const maxMemory = ref(4096)
  const totalMemoryMB = ref(0)
  const downloadMirror = ref('bmcl')
  const javaInstallations = ref<string[]>([])
  const hasFoundJavaInstallations = ref(false)

  async function loadSystemMemory() {
    try {
      totalMemoryMB.value = await configApi.getTotalMemory()
    } catch (err) {
      console.error('Failed to get total memory:', err)
    }
  }

  async function loadMaxMemory() {
    try {
      const memory = await configApi.loadConfigKey('maxMemory')
      if (memory) {
        maxMemory.value = parseInt(memory, 10)
      }
    } catch (err) {
      console.error('Failed to get max memory:', err)
    }
  }

  async function saveMaxMemory() {
    try {
      await configApi.saveConfigKey('maxMemory', maxMemory.value.toString())
    } catch (err) {
      console.error('Failed to set max memory:', err)
    }
  }

  async function loadDownloadMirror() {
    try {
      const mirror = await configApi.loadConfigKey('downloadMirror')
      if (mirror) {
        downloadMirror.value = mirror
      }
    } catch (err) {
      console.error('Failed to get download mirror:', err)
    }
  }

  async function saveDownloadMirror() {
    try {
      await configApi.saveConfigKey('downloadMirror', downloadMirror.value)
    } catch (err) {
      console.error('Failed to set download mirror:', err)
    }
  }

  async function findJavaInstallations() {
    try {
      const installations = await javaApi.findJavaInstallations()
      javaInstallations.value = installations
      hasFoundJavaInstallations.value = true
      return javaInstallations.value
    } catch (err) {
      console.error('Failed to find Java installations:', err)
      return []
    }
  }

  return {
    maxMemory,
    totalMemoryMB,
    downloadMirror,
    javaInstallations,
    hasFoundJavaInstallations,
    loadSystemMemory,
    loadMaxMemory,
    saveMaxMemory,
    loadDownloadMirror,
    saveDownloadMirror,
    findJavaInstallations
  }
})
