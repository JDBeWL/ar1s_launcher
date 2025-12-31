<template>
  <v-container fluid class="install-modpack-container pa-4">
    <v-card color="surface-container">
      <!-- 标题栏 -->
      <v-card-title class="d-flex align-center pa-4">
        <v-btn 
          variant="text" 
          icon="mdi-arrow-left" 
          :disabled="installing"
          @click="goBack" 
          class="mr-2"
        />
        <div class="flex-grow-1">
          <div class="d-flex align-center">
            <span class="text-h6">{{ title }}</span>
            <v-chip v-if="installing" color="primary" variant="tonal" size="small" class="ml-2">
              安装中
            </v-chip>
          </div>
          <div class="text-caption text-on-surface-variant">Modrinth · {{ projectId }}</div>
        </div>
      </v-card-title>

      <v-divider />

      <v-card-text class="pa-4">
        <!-- 安装进度 -->
        <v-expand-transition>
          <v-card v-if="installing" color="surface-container-high" class="mb-4" variant="flat">
            <v-card-text class="pa-4">
              <div class="d-flex align-center justify-space-between mb-2">
                <div class="d-flex align-center">
                  <v-progress-circular
                    v-if="installProgress.indeterminate"
                    indeterminate
                    size="18"
                    width="2"
                    color="primary"
                    class="mr-2"
                  />
                  <v-icon v-else size="18" color="primary" class="mr-2">mdi-download</v-icon>
                  <span class="text-body-2">{{ installProgress.message }}</span>
                </div>
                <span class="text-body-2 font-weight-medium text-primary">{{ installProgress.progress }}%</span>
              </div>
              <v-progress-linear
                :model-value="installProgress.progress"
                :indeterminate="installProgress.indeterminate"
                height="6"
                rounded
                color="primary"
                bg-color="surface-container"
              />
              <div class="d-flex justify-end mt-3">
                <v-btn
                  variant="text"
                  color="error"
                  size="small"
                  :loading="cancelling"
                  :disabled="cancelling"
                  @click="cancelInstall"
                >
                  <v-icon start size="16">mdi-close</v-icon>
                  取消
                </v-btn>
              </div>
            </v-card-text>
          </v-card>
        </v-expand-transition>

        <!-- 加载骨架屏 -->
        <template v-if="loadingVersions">
          <v-skeleton-loader type="text" class="mb-4" />
          <v-row>
            <v-col cols="12" md="6">
              <v-skeleton-loader type="text" />
            </v-col>
            <v-col cols="12" md="6">
              <v-skeleton-loader type="text" />
            </v-col>
          </v-row>
        </template>

        <!-- 版本选择表单 -->
        <template v-else-if="!installing">
          <!-- 无可用版本 -->
          <v-alert
            v-if="modpackVersions.length === 0"
            type="warning"
            variant="tonal"
            class="mb-4"
          >
            <template #title>无可用版本</template>
            该整合包暂无可安装的版本，请返回重新选择
          </v-alert>

          <template v-else>
            <!-- 版本配置区域 -->
            <div class="config-section mb-4">
              <div class="text-subtitle-2 text-on-surface-variant mb-3">
                <v-icon size="18" class="mr-1">mdi-tune</v-icon>
                版本配置
              </div>
              
              <v-row dense>
                <!-- 加载器 -->
                <v-col cols="6" sm="4" md="3">
                  <v-select
                    v-model="selectedLoader"
                    :items="loaderOptions"
                    label="加载器"
                    density="comfortable"
                    variant="outlined"
                    hide-details
                    @update:model-value="onLoaderChange"
                  >
                    <template #prepend-inner>
                      <v-icon size="18" color="on-surface-variant">mdi-puzzle</v-icon>
                    </template>
                  </v-select>
                </v-col>

                <!-- 游戏版本 -->
                <v-col cols="6" sm="4" md="3">
                  <v-select
                    v-model="selectedGameVersion"
                    :items="filteredGameVersions"
                    label="游戏版本"
                    density="comfortable"
                    variant="outlined"
                    :disabled="!selectedLoader"
                    clearable
                    hide-details
                    @update:model-value="onGameVersionChange"
                  >
                    <template #prepend-inner>
                      <v-icon size="18" color="on-surface-variant">mdi-minecraft</v-icon>
                    </template>
                  </v-select>
                </v-col>

                <!-- 整合包版本 -->
                <v-col cols="12" sm="4" md="6">
                  <v-select
                    v-model="selectedVersionId"
                    :items="filteredModpackVersions"
                    item-title="displayName"
                    item-value="id"
                    label="整合包版本"
                    density="comfortable"
                    variant="outlined"
                    :disabled="!selectedLoader"
                    hide-details
                  >
                    <template #prepend-inner>
                      <v-icon size="18" color="on-surface-variant">mdi-package-variant</v-icon>
                    </template>
                    <template #item="{ item, props }">
                      <v-list-item v-bind="props">
                        <template #subtitle>
                          <span class="text-caption">
                            {{ item.raw.game_versions?.slice(0, 3).join(', ') }}
                            {{ item.raw.game_versions?.length > 3 ? '...' : '' }}
                          </span>
                        </template>
                      </v-list-item>
                    </template>
                  </v-select>
                </v-col>
              </v-row>
            </div>

            <!-- 实例配置区域 -->
            <div class="config-section mb-4">
              <div class="text-subtitle-2 text-on-surface-variant mb-3">
                <v-icon size="18" class="mr-1">mdi-folder-cog</v-icon>
                实例配置
              </div>
              
              <v-text-field
                v-model="instanceName"
                label="实例名称"
                :placeholder="title"
                density="comfortable"
                variant="outlined"
                :error-messages="instanceNameError || undefined"
                :loading="checkingInstanceName"
                hide-details="auto"
                counter="64"
                maxlength="64"
              >
                <template #prepend-inner>
                  <v-icon size="18" color="on-surface-variant">mdi-rename</v-icon>
                </template>
                <template #append-inner>
                  <v-fade-transition>
                    <v-icon 
                      v-if="instanceName && !instanceNameError && !checkingInstanceName" 
                      size="18" 
                      color="success"
                    >
                      mdi-check-circle
                    </v-icon>
                  </v-fade-transition>
                </template>
              </v-text-field>
              
              <div class="text-caption text-on-surface-variant mt-2">
                留空将使用整合包名称作为实例名称
              </div>
            </div>

            <!-- 选中版本信息预览 -->
            <v-expand-transition>
              <v-card 
                v-if="selectedVersionInfo" 
                color="surface-container-high" 
                variant="flat"
                class="mb-4"
              >
                <v-card-text class="pa-3">
                  <div class="d-flex align-center justify-space-between">
                    <div>
                      <div class="text-body-2 font-weight-medium">{{ selectedVersionInfo.name }}</div>
                      <div class="text-caption text-on-surface-variant">
                        版本号: {{ selectedVersionInfo.version_number }}
                      </div>
                    </div>
                    <div class="d-flex ga-1">
                      <v-chip
                        v-for="loader in selectedVersionInfo.loaders.slice(0, 2)"
                        :key="loader"
                        size="x-small"
                        color="primary"
                        variant="tonal"
                      >
                        {{ loader }}
                      </v-chip>
                    </div>
                  </div>
                </v-card-text>
              </v-card>
            </v-expand-transition>
          </template>
        </template>

        <!-- 操作按钮 -->
        <div class="d-flex justify-end ga-2 mt-4">
          <v-btn
            v-if="!installing"
            variant="text"
            @click="goBack"
          >
            取消
          </v-btn>
          <v-btn 
            v-if="!installing"
            color="primary" 
            :disabled="!canInstall" 
            :loading="checkingInstanceName"
            @click="install"
          >
            <v-icon start>mdi-download</v-icon>
            安装
          </v-btn>
        </div>
      </v-card-text>
    </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useNotificationStore } from '../stores/notificationStore'

interface ModrinthVersion {
  id: string
  name: string
  version_number: string
  game_versions: string[]
  loaders: string[]
}

interface InstallProgress {
  progress: number
  message: string
  indeterminate: boolean
}

interface InstanceNameValidation {
  is_valid: boolean
  error_message: string | null
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
const instanceName = ref<string>('')
const installing = ref(false)
const cancelling = ref(false)
const installProgress = ref<InstallProgress>({
  progress: 0,
  message: '准备安装...',
  indeterminate: false
})

const instanceNameError = ref<string | null>(null)
const checkingInstanceName = ref(false)

let unlistenProgress: UnlistenFn | null = null
let instanceNameCheckTimeout: ReturnType<typeof setTimeout> | null = null

// 获取选中版本的详细信息
const selectedVersionInfo = computed(() => {
  if (!selectedVersionId.value) return null
  return modpackVersions.value.find(v => v.id === selectedVersionId.value) || null
})

// 获取实际使用的实例名称
const effectiveInstanceName = computed(() => {
  return instanceName.value.trim() || title
})

// 监听实例名称变化
watch(instanceName, (newName) => {
  if (instanceNameCheckTimeout) {
    clearTimeout(instanceNameCheckTimeout)
  }
  
  instanceNameError.value = null
  
  // 使用实际名称进行检查
  const nameToCheck = newName.trim() || title
  if (!nameToCheck) return
  
  instanceNameCheckTimeout = setTimeout(async () => {
    await checkInstanceName(nameToCheck)
  }, 300)
})

async function checkInstanceName(name: string) {
  if (!name) return
  
  checkingInstanceName.value = true
  try {
    const result = await invoke('check_instance_name_available', { name }) as InstanceNameValidation
    instanceNameError.value = result.is_valid ? null : result.error_message
  } catch (e) {
    console.error('检查实例名称失败:', e)
  } finally {
    checkingInstanceName.value = false
  }
}

const canInstall = computed(() => {
  return selectedVersionId.value && 
         !instanceNameError.value && 
         !checkingInstanceName.value &&
         modpackVersions.value.length > 0
})

onBeforeRouteLeave(() => {
  if (installing.value && !cancelling.value) {
    notificationStore.warning('安装进行中', '请先取消安装或等待安装完成')
    return false
  }
  return true
})

function goBack() {
  if (installing.value && !cancelling.value) {
    notificationStore.warning('安装进行中', '请先取消安装或等待安装完成')
    return
  }
  router.back()
}

async function cancelInstall() {
  if (cancelling.value) return
  cancelling.value = true
  try {
    await invoke('cancel_modpack_install')
    notificationStore.info('正在取消', '安装将在当前步骤完成后取消')
  } catch (e) {
    console.error('取消安装失败:', e)
  }
}

const loaderOptions = computed(() => {
  const loaders = new Set<string>()
  for (const v of modpackVersions.value) {
    for (const loader of v.loaders) {
      loaders.add(loader)
    }
  }
  const priority = ['forge', 'neoforge', 'fabric', 'quilt']
  return Array.from(loaders).sort((a, b) => {
    const ai = priority.indexOf(a.toLowerCase())
    const bi = priority.indexOf(b.toLowerCase())
    return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi)
  })
})

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

const filteredModpackVersions = computed(() => {
  if (!selectedLoader.value) return []
  
  let filtered = modpackVersions.value.filter(v => 
    v.loaders.some(l => l.toLowerCase() === selectedLoader.value?.toLowerCase())
  )
  
  if (selectedGameVersion.value) {
    filtered = filtered.filter(v => v.game_versions.includes(selectedGameVersion.value as string))
  }
  
  return filtered.map(v => ({
    ...v,
    displayName: `${v.name} (${v.version_number})`
  }))
})

function onLoaderChange() {
  selectedGameVersion.value = filteredGameVersions.value[0] || null
  onGameVersionChange()
}

function onGameVersionChange() {
  const first = filteredModpackVersions.value[0]
  selectedVersionId.value = first ? first.id : null
}

async function loadVersions() {
  if (!projectId) return
  loadingVersions.value = true
  try {
    const versions = await invoke('get_modrinth_modpack_versions', {
      projectId,
      gameVersions: undefined,
      loaders: undefined,
    }) as ModrinthVersion[]
    modpackVersions.value = versions || []

    if (loaderOptions.value.length > 0) {
      selectedLoader.value = loaderOptions.value[0]
      onLoaderChange()
    }
  } catch (e) {
    console.error('加载整合包版本失败:', e)
    notificationStore.error('加载失败', '无法获取整合包版本信息')
  } finally {
    loadingVersions.value = false
  }
}

async function install() {
  if (!projectId || !selectedVersionId.value || !canInstall.value) return
  
  installing.value = true
  installProgress.value = { progress: 0, message: '准备安装...', indeterminate: false }
  
  try {
    unlistenProgress = await listen<InstallProgress>('modpack-install-progress', (event) => {
      installProgress.value = event.payload
    })

    await invoke('install_modrinth_modpack', {
      options: {
        modpack_id: projectId,
        version_id: selectedVersionId.value,
        instance_name: effectiveInstanceName.value,
        install_path: '',
      }
    })
    
    notificationStore.success('安装成功', `${effectiveInstanceName.value} 已安装完成`)
    router.push('/instance-manager')
  } catch (e) {
    console.error('安装整合包失败:', e)
    const errorMessage = e instanceof Error ? e.message : String(e)
    notificationStore.error('安装失败', errorMessage, true)
  } finally {
    installing.value = false
    cancelling.value = false
    if (unlistenProgress) {
      unlistenProgress()
      unlistenProgress = null
    }
  }
}

onMounted(async () => {
  await loadVersions()
  // 检查默认实例名称
  if (title) {
    await checkInstanceName(title)
  }
})

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress()
  }
  if (instanceNameCheckTimeout) {
    clearTimeout(instanceNameCheckTimeout)
  }
})

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

<style scoped>
.install-modpack-container {
  max-width: 900px;
  margin: 0 auto;
}

.config-section {
  padding: 16px;
  background: rgb(var(--v-theme-surface-container-low));
  border-radius: 12px;
}
</style>
