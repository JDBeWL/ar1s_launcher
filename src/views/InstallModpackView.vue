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
import { useNotificationStore } from '../stores/notificationStore'

interface ModrinthVersion {
  id: string;
  name: string;
  game_versions: string[];
}

const route = useRoute()
const router = useRouter()
const notificationStore = useNotificationStore()

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
    notificationStore.success('安装成功', '整合包已安装完成')
    router.push('/instance-manager')
  } catch (e) {
    console.error('安装整合包失败:', e)
    const errorMessage = e instanceof Error ? e.message : String(e)
    notificationStore.error('安装失败', errorMessage, true)
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

/**
 * 比较 Minecraft 版本号（降序）
 * 支持格式：1.20.1, 1.20.1-pre1, 1.20.1-rc1, 24w10a (快照)
 */
function compareVersionDesc(a: string, b: string): number {
  // 解析版本号，提取主版本和后缀
  const parseVersion = (v: string) => {
    // 匹配快照格式 (如 24w10a)
    const snapshotMatch = v.match(/^(\d+)w(\d+)([a-z])$/)
    if (snapshotMatch) {
      return {
        type: 'snapshot',
        parts: [parseInt(snapshotMatch[1]), parseInt(snapshotMatch[2])],
        suffix: snapshotMatch[3]
      }
    }
    
    // 匹配正式版/预览版格式 (如 1.20.1, 1.20.1-pre1, 1.20.1-rc1)
    const match = v.match(/^([\d.]+)(?:-(.+))?$/)
    if (!match) return { type: 'unknown', parts: [0], suffix: v }
    
    const parts = match[1].split('.').map(n => parseInt(n) || 0)
    const suffix = match[2] || ''
    
    // 确定版本类型优先级：release > rc > pre
    let type = 'release'
    let suffixNum = 0
    if (suffix.startsWith('rc')) {
      type = 'rc'
      suffixNum = parseInt(suffix.slice(2)) || 0
    } else if (suffix.startsWith('pre')) {
      type = 'pre'
      suffixNum = parseInt(suffix.slice(3)) || 0
    } else if (suffix) {
      type = 'other'
    }
    
    return { type, parts, suffix, suffixNum }
  }
  
  const va = parseVersion(a)
  const vb = parseVersion(b)
  
  // 快照版本排在正式版之后
  if (va.type === 'snapshot' && vb.type !== 'snapshot') return 1
  if (vb.type === 'snapshot' && va.type !== 'snapshot') return -1
  
  // 比较主版本号
  const maxLen = Math.max(va.parts.length, vb.parts.length)
  for (let i = 0; i < maxLen; i++) {
    const av = va.parts[i] ?? 0
    const bv = vb.parts[i] ?? 0
    if (av !== bv) return bv - av
  }
  
  // 主版本相同，比较类型优先级
  const typePriority: Record<string, number> = { release: 3, rc: 2, pre: 1, other: 0, snapshot: -1, unknown: -2 }
  const typeDiff = (typePriority[vb.type] ?? 0) - (typePriority[va.type] ?? 0)
  if (typeDiff !== 0) return typeDiff
  
  // 类型相同，比较后缀数字
  return (vb.suffixNum ?? 0) - (va.suffixNum ?? 0)
}
</script>

<style scoped>
</style>


