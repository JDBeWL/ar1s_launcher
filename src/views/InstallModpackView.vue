<template>
  <v-container>
    <v-card>
      <v-card-title class="d-flex align-center">
        安装整合包
        <v-spacer></v-spacer>
        <v-btn variant="text" icon="mdi-arrow-left" @click="goBack"></v-btn>
      </v-card-title>
      <v-card-text>
        <v-row class="mb-4">
          <v-col cols="12">
            <div class="text-h6">{{ title }}</div>
            <div class="text-caption text-grey">项目 ID: {{ projectId }}</div>
          </v-col>
        </v-row>

        <v-row no-gutters class="align-center mb-4">
          <v-col class="shrink pr-2" style="max-width: 200px">
            <v-select
              v-model="selectedGameVersion"
              :items="gameVersionOptions"
              label="游戏版本"
              :loading="loadingVersions"
              clearable
              hide-details
              @update:model-value="onGameVersionChange"
            ></v-select>
          </v-col>
          <v-col class="shrink pr-2" style="max-width: 280px">
            <v-select
              v-model="selectedVersionId"
              :items="filteredModpackVersions"
              item-title="name"
              item-value="id"
              label="整合包版本"
              :loading="loadingVersions"
              hide-details
            ></v-select>
          </v-col>
          <v-col class="flex-grow-1 pl-2">
            <v-text-field
              v-model="instanceName"
              :label="`实例名称 (默认: ${title})`"
              :placeholder="title"
              hide-details
            ></v-text-field>
          </v-col>
        </v-row>

        <v-row>
          <v-col cols="12" class="text-right">
            <v-btn color="primary" size="large" :disabled="!selectedVersionId || installing" :loading="installing" @click="install">
              安装整合包
            </v-btn>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

interface ModrinthVersion {
  id: string;
  name: string;
  game_versions: string[];
}

const route = useRoute()
const router = useRouter()

const projectId = String(route.query.projectId || '')
const title = String(route.query.title || '')

const loadingVersions = ref(false)
const modpackVersions = ref<ModrinthVersion[]>([])
const selectedVersionId = ref<string | null>(null)
const selectedGameVersion = ref<string | null>(null)
const gameVersionOptions = ref<string[]>([])
const instanceName = ref<string>(title)
const installing = ref(false)

function goBack() {
  router.back()
}

async function loadVersions() {
  if (!projectId) return
  loadingVersions.value = true
  try {
    const versions = await invoke('get_modrinth_modpack_versions', {
      projectId: projectId,
      gameVersions: undefined,
      loaders: undefined,
    }) as ModrinthVersion[]
    modpackVersions.value = versions || []
    // 生成游戏版本选项（去重并按版本号降序）
    const allGameVersions = new Set<string>()
    for (const v of modpackVersions.value) {
      for (const gv of (v.game_versions || [])) allGameVersions.add(gv)
    }
    gameVersionOptions.value = Array.from(allGameVersions).sort(compareVersionDesc)

    // 默认选择最新游戏版本
    if (gameVersionOptions.value.length > 0) {
      selectedGameVersion.value = gameVersionOptions.value[0]
    }

    // 根据当前选择的游戏版本，选择第一个匹配的整合包版本
    const first = filteredModpackVersions.value[0]
    selectedVersionId.value = first ? first.id : null
  } catch (e) {
    console.error('加载整合包版本失败:', e)
  } finally {
    loadingVersions.value = false
  }
}

async function install() {
  if (!projectId || !selectedVersionId.value) return
  installing.value = true
  try {
    const options = {
      modpack_id: projectId,
      version_id: selectedVersionId.value,
      instance_name: instanceName.value || title,
      install_path: '',
    }
    await invoke('install_modrinth_modpack', options)
    alert('整合包安装成功')
    router.push('/instance-manager')
  } catch (e) {
    console.error('安装整合包失败:', e)
    alert('安装失败')
  } finally {
    installing.value = false
  }
}

onMounted(() => {
  loadVersions()
})

// 计算属性：按所选游戏版本过滤整合包版本
const filteredModpackVersions = computed(() => {
  if (!selectedGameVersion.value) return modpackVersions.value
  return modpackVersions.value.filter(v => (v.game_versions || []).includes(selectedGameVersion.value as string))
})

function onGameVersionChange() {
  const first = filteredModpackVersions.value[0]
  selectedVersionId.value = first ? first.id : null
}

function compareVersionDesc(a: string, b: string): number {
  const ap = a.split('.').map(n => parseInt(n || '0', 10))
  const bp = b.split('.').map(n => parseInt(n || '0', 10))
  const len = Math.max(ap.length, bp.length)
  for (let i = 0; i < len; i++) {
    const av = ap[i] ?? 0
    const bv = bp[i] ?? 0
    if (av !== bv) return bv - av
  }
  return 0
}
</script>

<style scoped>
</style>


