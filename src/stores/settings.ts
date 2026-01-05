import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useSettingsStore = defineStore('settings', () => {
  const maxMemory = ref(4096)
  const totalMemoryMB = ref(0)
  const downloadMirror = ref('bmcl')
  const javaInstallations = ref<string[]>([])
  const hasFoundJavaInstallations = ref(false)

  async function loadSystemMemory() {
    try {
      const memoryBytes = await invoke('get_total_memory') as number
      totalMemoryMB.value = Math.round(memoryBytes / 1024 / 1024)
    } catch (err) {
      console.error('Failed to get total memory:', err)
    }
  }

  async function loadMaxMemory() {
    try {
      const memory = await invoke('load_config_key', { key: 'maxMemory' })
      if (memory) {
        maxMemory.value = parseInt(memory as string, 10)
      }
    } catch (err) {
      console.error('Failed to get max memory:', err)
    }
  }

  async function saveMaxMemory() {
    try {
      await invoke('save_config_key', { key: 'maxMemory', value: maxMemory.value.toString() })
    } catch (err) {
      console.error('Failed to set max memory:', err)
    }
  }

  async function loadDownloadMirror() {
    try {
      const mirror = await invoke('load_config_key', { key: 'downloadMirror' })
      if (mirror) {
        downloadMirror.value = mirror as string
      }
    } catch (err) {
      console.error('Failed to get download mirror:', err)
    }
  }

  async function saveDownloadMirror() {
    try {
      await invoke('save_config_key', { key: 'downloadMirror', value: downloadMirror.value })
    } catch (err) {
      console.error('Failed to set download mirror:', err)
    }
  }

  async function findJavaInstallations() {
    try {
      const installations = await invoke('find_java_installations_command')
      javaInstallations.value = installations as string[]
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