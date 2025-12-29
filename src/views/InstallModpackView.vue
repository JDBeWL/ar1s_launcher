<template>
  <v-container fluid class="pa-4">
    <v-card color="surface-container">
      <v-card-title class="d-flex align-center pa-4">
        <v-btn variant="text" icon="mdi-arrow-left" @click="goBack" class="mr-2"></v-btn>
        <span>安装整合包</span>
      </v-card-title>
      <v-card-text class="pa-4">
        <v-row class="mb-4">
          <v-col cols="12">
            <div class="text-h6">{{ title }}</div>
            <div class="text-caption text-on-surface-variant">项目 ID: {{ projectId }}</div>
          </v-col>
        </v-row>

        <!-- 安装进度 -->
        <v-card v-if="installing" color="surface-container-high" class="mb-4">
          <v-card-text class="pa-4">
            <div class="d-flex align-center justify-space-between mb-2">
              <span class="text-body-2">{{ installProgress.message }}</span>
              <span class="text-body-2 font-weight-medium">{{ installProgress.progress }}%</span>
            </div>
            <v-progress-linear
              :model-value="installProgress.progress"
              :indeterminate="installProgress.indeterminate"
              height="8"
              rounded
              color="primary"
            />
          </v-card-text>
        </v-card>

        <!-- 加载版本中 -->
        <div v-if="loadingVersions && !installing" class="d-flex align-center mb-4">
          <v-progress-circular indeterminate size="20" width="2" color="primary" class="mr-3" />
          <span class="text-body-2 text-on-surface-variant">加载版本信息...</span>
        </div>

        <v-row no-gutters class="align-center mb-4" v-if="!installing && !loadingVersions">
          <!-- 加载器选择 -->
          <v-col class="shrink pr-2" style="max-width: 150px">
            <v-select
              v-model="selectedLoader"
              :items="loaderOptions"
              label="加载器"
              hide-details
              @update:model-value="onLoaderChange"
            ></v-select>
          </v-col>

          <!-- 游戏版本选择 -->
          <v-col class="shrink pr-2" style="max-width: 150px">
            <v-select
              v-model="selectedGameVersion"
              :items="filteredGameVersions"
              label="游戏版本"
              :disabled="!selectedLoader"
              clearable
              hide-details
              @update:model-value="onGameVersionChange"
            ></v-select>
          </v-col>

          <!-- 整合包版本选择 -->
          <v-col class="shrink pr-2" style="min-width: 200px">
            <v-select
              v-model="selectedVersionId"
              :items="filteredModpackVersions"
              item-title="displayName"
              item-value="id"
              label="整合包版本"
              :disabled="!selectedLoader"
              hide-details
            ></v-select>
          </v-col>

          <!-- 实例名称 -->
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
            <v-btn 
              v-if="!installing"
              color="primary" 
              size="large" 
              :disabled="!selectedVersionId" 
              @click="install"
            >
              <v-icon start>mdi-download</v-icon>
              安装整合包
            </v-btn>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useNotificationStore } from '../stores/notificationStore'

interface ModrinthVersion {
  id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
}

interface InstallProgress {
  progress: number;
  message: string;
  indeterminate: boolean;
}

const route = useRoute()
const router = useRouter()
const notificationStore = useNotificationStore()

const projectId = String(route.query.projectId || '')
const title = String(route.query.title || '')

const loadingVersions = ref(false)
const modpackVersions = ref<ModrinthVersion[]>([])
const selectedVersionId = ref<string | null>(null)
const selectedLoader = ref<string | null>(null)
const selectedGameVersion = ref<string | null>(null)
const instanceName = ref<string>(title)
const installing = ref(false)
const installProgress = ref<InstallProgress>({
  progress: 0,
  message: '准备安装...',
  indeterminate: false
})

let unlistenProgress: UnlistenFn | null = null

function goBack() {
  router.back()
}

// 获取所有可用的加载器类型
const loaderOptions = computed(() => {
  const loaders = new Set<string>()
  for (const v of modpackVersions.value) {
    for (const loader of v.loaders) {
      loaders.add(loader)
    }
  }
  // 按优先级排序
  const priority = ['forge', 'neoforge', 'fabric', 'quilt']
  return Array.from(loaders).sort((a, b) => {
    const ai = priority.indexOf(a.toLowerCase())
    const bi = priority.indexOf(b.toLowerCase())
    return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi)
  })
})

// 根据选择的加载器过滤游戏版本
const filteredGameVersions = computed(() => {
  if (!selectedLoader.value) return []
  
  const versions = new Set<string>()
  for (const v of modpackVersions.value) {
    if (v.loaders.some(l => l.toLowerCase() === selectedLoader.value?.toLowerCase())) {
      for (const gv of v.game_versions) {
        versions.add(gv)
      }
    }
  }
  return Array.from(versions).sort(compareVersionDesc)
})

// 根据选择的加载器和游戏版本过滤整合包版本
const filteredModpackVersions = computed(() => {
  if (!selectedLoader.value) return []
  
  let filtered = modpackVersions.value.filter(v => 
    v.loaders.some(l => l.toLowerCase() === selectedLoader.value?.toLowerCase())
  )
  
  if (selectedGameVersion.value) {
    filtered = filtered.filter(v => v.game_versions.includes(selectedGameVersion.value as string))
  }
  
  // 添加显示名称
  return filtered.map(v => ({
    ...v,
    displayName: `${v.name} (${v.version_number})`
  }))
})

function onLoaderChange() {
  // 重置游戏版本和整合包版本
  selectedGameVersion.value = filteredGameVersions.value[0] || null
  onGameVersionChange()
}

function onGameVersionChange() {
  // 选择第一个匹配的整合包版本
  const first = filteredModpackVersions.value[0]
  selectedVersionId.value = first ? first.id : null
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

    // 默认选择第一个加载器
    if (loaderOptions.value.length > 0) {
      selectedLoader.value = loaderOptions.value[0]
      onLoaderChange()
    }
  } catch (e) {
    console.error('加载整合包版本失败:', e)
  } finally {
    loadingVersions.value = false
  }
}

async function install() {
  if (!projectId || !selectedVersionId.value) return
  installing.value = true
  installProgress.value = { progress: 0, message: '准备安装...', indeterminate: false }
  
  try {
    // 监听安装进度
    unlistenProgress = await listen<InstallProgress>('modpack-install-progress', (event) => {
      installProgress.value = event.payload
    })

    const options = {
      modpack_id: projectId,
      version_id: selectedVersionId.value,
      instance_name: instanceName.value || title,
      install_path: '',
    }
    await invoke('install_modrinth_modpack', { options })
    notificationStore.success('安装成功', '整合包已安装完成')
    router.push('/instance-manager')
  } catch (e) {
    console.error('安装整合包失败:', e)
    const errorMessage = e instanceof Error ? e.message : String(e)
    notificationStore.error('安装失败', errorMessage, true)
  } finally {
    installing.value = false
    if (unlistenProgress) {
      unlistenProgress()
      unlistenProgress = null
    }
  }
}

onMounted(() => {
  loadVersions()
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
})

/**
 * 比较 Minecraft 版本号（降序）
 */
function compareVersionDesc(a: string, b: string): number {
  const parseVersion = (v: string) => {
    const snapshotMatch = v.match(/^(\d+)w(\d+)([a-z])$/)
    if (snapshotMatch) {
      return { type: 'snapshot', parts: [parseInt(snapshotMatch[1]), parseInt(snapshotMatch[2])], suffix: snapshotMatch[3], suffixNum: 0 }
    }
    
    const match = v.match(/^([\d.]+)(?:-(.+))?$/)
    if (!match) return { type: 'unknown', parts: [0], suffix: v, suffixNum: 0 }
    
    const parts = match[1].split('.').map(n => parseInt(n) || 0)
    const suffix = match[2] || ''
    
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
  
  if (va.type === 'snapshot' && vb.type !== 'snapshot') return 1
  if (vb.type === 'snapshot' && va.type !== 'snapshot') return -1
  
  const maxLen = Math.max(va.parts.length, vb.parts.length)
  for (let i = 0; i < maxLen; i++) {
    const av = va.parts[i] ?? 0
    const bv = vb.parts[i] ?? 0
    if (av !== bv) return bv - av
  }
  
  const typePriority: Record<string, number> = { release: 3, rc: 2, pre: 1, other: 0, snapshot: -1, unknown: -2 }
  const typeDiff = (typePriority[vb.type] ?? 0) - (typePriority[va.type] ?? 0)
  if (typeDiff !== 0) return typeDiff
  
  return (vb.suffixNum ?? 0) - (va.suffixNum ?? 0)
}
</script>


