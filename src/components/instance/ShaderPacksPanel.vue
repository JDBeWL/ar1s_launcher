<script setup lang="ts">
/**
 * 实例详情页的光影包面板
 * 光影包没有模组那种 .disabled 约定，选中哪个由游戏自己的配置决定：
 * Iris / Oculus 读 config/iris.properties，OptiFine 读 optionsshaders.txt。
 * 所以这里只改配置，不挪文件；同一时间只有一个包是启用的。
 */
import { computed, onMounted, ref, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import ShaderPackBrowserDialog from './ShaderPackBrowserDialog.vue';
import { useNotificationStore } from '../../stores/notificationStore';
import { configApi, shaderPackApi } from '../../services';
import type { ShaderBackend, ShaderPackInfo, ShaderPackList, ShaderPackUpdate } from '../../types/events';
import { formatFileSize, getErrorMessage } from '../../utils/format';
import { logError } from '../../utils/logger';

const props = defineProps<{
  instanceName: string;
  gameVersion?: string | null;
}>();

const emit = defineEmits<{
  (e: 'changed'): void;
}>();

const notificationStore = useNotificationStore();

const state = ref<ShaderPackList | null>(null);
const loading = ref(false);
const error = ref('');
const busyFile = ref<string | null>(null);
const toggling = ref(false);
const importing = ref(false);
const deleteDialog = ref(false);
const packToDelete = ref<ShaderPackInfo | null>(null);
const browser = ref(false);
const updates = ref<ShaderPackUpdate[]>([]);
const checking = ref(false);
const updatingFile = ref<string | null>(null);
const updatingAll = ref(false);
// 关闭版本隔离时 shaderpacks 放在共享目录，选择会影响所有实例
const versionIsolation = ref(true);

const packs = computed(() => state.value?.packs ?? []);
const selectedName = computed(() => state.value?.selected ?? null);
const backend = computed<ShaderBackend>(() => state.value?.backend ?? 'unknown');
const supportsToggle = computed(() => state.value?.supportsToggle ?? false);
const shadersEnabled = computed(() => state.value?.enabled ?? false);

const backendLabel = computed(() => {
  switch (backend.value) {
    case 'iris':
      return 'Iris';
    case 'oculus':
      return 'Oculus';
    case 'optifine':
      return 'OptiFine';
    default:
      return '未检测到光影加载器';
  }
});

const updateMap = computed(() => {
  const map = new Map<string, ShaderPackUpdate>();
  for (const update of updates.value) {
    map.set(update.fileName, update);
  }
  return map;
});

const updatableCount = computed(() => updates.value.length);

/** 配置里选中的包已经不在磁盘上：游戏会退回无光影，得在界面上说清楚 */
const selectedPack = computed(() => {
  const name = selectedName.value?.toLowerCase();
  if (!name) return null;
  // 配置里的名字大小写未必和磁盘一致，和后端的判定保持一致
  return packs.value.find((pack) => pack.fileName.toLowerCase() === name) ?? null;
});

const missingSelection = computed(() => (selectedName.value && !selectedPack.value ? selectedName.value : null));

const invalidCount = computed(() => packs.value.filter((pack) => !pack.valid).length);

/** 表头优先显示包名而不是文件名；包不在磁盘上时退回配置里记的原值 */
const selectedDisplayName = computed(() => selectedPack.value?.displayName ?? selectedName.value);

function backendHint(): string {
  switch (backend.value) {
    case 'iris':
      return '选择即时生效，游戏下次启动或按 K 重载光影时读取 iris.properties';
    case 'oculus':
      return '选择即时生效，游戏下次启动或按 K 重载光影时读取 oculus.properties';
    case 'optifine':
      return '选择即时生效，游戏下次启动读取 optionsshaders.txt；关闭光影写 shaderPack=OFF';
    default:
      return '光影包已放入 shaderpacks，但游戏不会加载它们：先在「模组管理」里安装 Iris / Oculus 或 OptiFine';
  }
}

async function loadPacks() {
  loading.value = true;
  error.value = '';
  try {
    state.value = await shaderPackApi.getShaderPacks(props.instanceName);
  } catch (e) {
    logError('Failed to load shader packs', e, 'ShaderPacksPanel');
    error.value = getErrorMessage(e);
  } finally {
    loading.value = false;
  }
}

async function loadIsolationSetting() {
  try {
    const value = await configApi.loadConfigKey('versionIsolation');
    if (value !== null) versionIsolation.value = value === 'true';
  } catch (e) {
    logError('Failed to load isolation setting', e, 'ShaderPacksPanel');
  }
}

async function selectPack(pack: ShaderPackInfo) {
  if (busyFile.value || pack.selected) return;

  busyFile.value = pack.fileName;
  try {
    state.value = await shaderPackApi.selectShaderPack(props.instanceName, pack.fileName);
    emit('changed');
    notificationStore.success('已启用光影', pack.displayName);
  } catch (e) {
    logError('Failed to select shader pack', e, 'ShaderPacksPanel');
    notificationStore.error('启用光影失败', getErrorMessage(e), true);
  } finally {
    busyFile.value = null;
  }
}

async function toggleShaders() {
  if (toggling.value || !state.value) return;

  toggling.value = true;
  try {
    state.value = await shaderPackApi.setShadersEnabled(props.instanceName, !shadersEnabled.value);
    emit('changed');
  } catch (e) {
    logError('Failed to toggle shaders', e, 'ShaderPacksPanel');
    notificationStore.error('切换光影开关失败', getErrorMessage(e), true);
  } finally {
    toggling.value = false;
  }
}

async function disableShaders() {
  if (busyFile.value) return;

  busyFile.value = selectedName.value ?? '__off__';
  try {
    state.value = await shaderPackApi.disableShaderPack(props.instanceName);
    emit('changed');
    notificationStore.success('已关闭光影', '文件仍留在 shaderpacks 目录里');
  } catch (e) {
    logError('Failed to disable shaders', e, 'ShaderPacksPanel');
    notificationStore.error('关闭光影失败', getErrorMessage(e), true);
  } finally {
    busyFile.value = null;
  }
}

function askDeletePack(pack: ShaderPackInfo) {
  packToDelete.value = pack;
  deleteDialog.value = true;
}

async function deletePack() {
  if (!packToDelete.value) return;

  const fileName = packToDelete.value.fileName;
  busyFile.value = fileName;
  try {
    await shaderPackApi.deleteShaderPack(props.instanceName, fileName);
    deleteDialog.value = false;
    packToDelete.value = null;
    updates.value = updates.value.filter((item) => item.fileName !== fileName);
    await loadPacks();
    emit('changed');
  } catch (e) {
    logError('Failed to delete shader pack', e, 'ShaderPacksPanel');
    notificationStore.error('删除光影包失败', getErrorMessage(e));
  } finally {
    busyFile.value = null;
  }
}

async function importPacks() {
  if (importing.value) return;

  try {
    const selected = await open({
      multiple: true,
      title: '选择光影包',
      filters: [{ name: '光影包', extensions: ['zip'] }],
    });
    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;

    importing.value = true;
    const summary = await shaderPackApi.importShaderPacks(props.instanceName, paths);
    await loadPacks();
    emit('changed');

    if (summary.imported.length === 0) {
      notificationStore.warning(
        '没有导入任何光影包',
        summary.skipped.length > 0 ? `已跳过：${summary.skipped.join('、')}` : '所选文件里没有 shaders 目录',
      );
    } else if (summary.skipped.length > 0) {
      notificationStore.warning(`导入了 ${summary.imported.length} 个光影包`, `跳过：${summary.skipped.join('、')}`);
    } else {
      notificationStore.success('导入完成', `${summary.imported.length} 个光影包已放入 shaderpacks`);
    }
  } catch (e) {
    logError('Failed to import shader packs', e, 'ShaderPacksPanel');
    notificationStore.error('导入光影包失败', getErrorMessage(e), true);
  } finally {
    importing.value = false;
  }
}

async function openFolder() {
  try {
    await shaderPackApi.openFolder(props.instanceName);
  } catch (e) {
    logError('Failed to open shaderpacks folder', e, 'ShaderPacksPanel');
    notificationStore.error('打开文件夹失败', getErrorMessage(e));
  }
}

function openBrowser() {
  browser.value = true;
}

async function onInstalled() {
  await loadPacks();
  emit('changed');
}

async function checkUpdates() {
  if (checking.value) return;

  checking.value = true;
  try {
    updates.value = await shaderPackApi.checkUpdates(props.instanceName);
    if (updates.value.length === 0) {
      notificationStore.success('已是最新', '没有检测到可更新的光影包');
    } else {
      notificationStore.info('发现更新', `${updates.value.length} 个光影包有新版本`);
    }
  } catch (e) {
    logError('Failed to check shader pack updates', e, 'ShaderPacksPanel');
    notificationStore.error('检查更新失败', getErrorMessage(e), true);
  } finally {
    checking.value = false;
  }
}

async function updatePack(pack: ShaderPackInfo) {
  const update = updateMap.value.get(pack.fileName);
  if (!update || updatingFile.value) return;

  updatingFile.value = pack.fileName;
  try {
    const installed = await shaderPackApi.updateShaderPack(props.instanceName, pack.fileName, update.versionId);
    updates.value = updates.value.filter((item) => item.fileName !== pack.fileName);
    await loadPacks();
    emit('changed');
    notificationStore.success('更新成功', `${pack.displayName} 已更新到 ${installed.versionNumber}`);
  } catch (e) {
    logError('Failed to update shader pack', e, 'ShaderPacksPanel');
    notificationStore.error('更新失败', getErrorMessage(e), true);
  } finally {
    updatingFile.value = null;
  }
}

async function updateAll() {
  if (updatingAll.value) return;

  const pending = [...updates.value];
  if (pending.length === 0) return;

  updatingAll.value = true;
  let succeeded = 0;
  let failed = 0;
  try {
    for (const update of pending) {
      try {
        await shaderPackApi.updateShaderPack(props.instanceName, update.fileName, update.versionId);
        succeeded += 1;
        updates.value = updates.value.filter((item) => item.fileName !== update.fileName);
      } catch (e) {
        failed += 1;
        logError(`Failed to update ${update.fileName}`, e, 'ShaderPacksPanel');
      }
    }

    await loadPacks();
    emit('changed');

    if (failed === 0) {
      notificationStore.success('更新完成', `${succeeded} 个光影包已更新`);
    } else {
      notificationStore.warning('部分更新失败', `成功 ${succeeded} 个，失败 ${failed} 个`);
    }
  } finally {
    updatingAll.value = false;
  }
}

onMounted(() => {
  void loadPacks();
  void loadIsolationSetting();
});

// 在实例之间切换时组件会被复用，必须按新实例重新拉列表
watch(
  () => props.instanceName,
  () => {
    updates.value = [];
    void loadPacks();
  },
);
</script>

<template>
  <div>
    <div class="d-flex align-center ga-2 mb-3 flex-wrap">
      <v-btn color="primary" variant="tonal" size="small" prepend-icon="mdi-plus" @click="openBrowser">
        安装光影包
      </v-btn>
      <v-btn
        variant="text"
        size="small"
        prepend-icon="mdi-folder-open-outline"
        :loading="importing"
        @click="importPacks"
      >
        从文件导入
      </v-btn>
      <v-btn variant="text" size="small" prepend-icon="mdi-update" :loading="checking" @click="checkUpdates">
        检查更新
      </v-btn>
      <v-btn icon size="small" variant="text" aria-label="刷新光影包列表" :loading="loading" @click="loadPacks">
        <v-icon size="18">mdi-refresh</v-icon>
      </v-btn>

      <v-spacer />

      <v-btn
        v-if="updatableCount > 0"
        color="primary"
        variant="tonal"
        size="small"
        :loading="updatingAll"
        @click="updateAll"
      >
        全部更新 ({{ updatableCount }})
      </v-btn>
    </div>

    <v-card variant="flat" color="surface-container" class="mb-3">
      <v-card-text class="pa-4">
        <div class="d-flex align-center flex-wrap ga-3">
          <v-avatar size="40" rounded="lg" color="surface-container-high">
            <v-icon size="20" color="on-surface-variant">mdi-shimmer</v-icon>
          </v-avatar>

          <div class="flex-grow-1 min-width-0">
            <div class="text-title-small">
              <template v-if="shadersEnabled && selectedDisplayName"> 当前光影：{{ selectedDisplayName }} </template>
              <template v-else-if="shadersEnabled">光影已开启，但还没选择光影包</template>
              <template v-else>光影已关闭</template>
            </div>
            <div class="text-body-small text-on-surface-variant">加载器：{{ backendLabel }} · {{ backendHint() }}</div>
          </div>

          <div class="d-flex align-center ga-1">
            <v-switch
              v-if="supportsToggle"
              :model-value="shadersEnabled"
              :loading="toggling"
              :disabled="toggling || packs.length === 0"
              aria-label="光影总开关"
              label="启用光影"
              hide-details
              inset
              density="compact"
              color="primary"
              @update:model-value="toggleShaders"
            />
            <v-btn
              v-else-if="shadersEnabled"
              variant="text"
              size="small"
              :loading="busyFile !== null"
              @click="disableShaders"
            >
              关闭光影
            </v-btn>
          </div>
        </div>

        <v-alert v-if="missingSelection" type="warning" variant="tonal" density="compact" class="mt-3">
          配置里选中的光影包 <strong>{{ missingSelection }}</strong> 已经不在 shaderpacks 目录里，游戏会退回无光影。
        </v-alert>
        <v-alert v-else-if="backend === 'unknown'" type="info" variant="tonal" density="compact" class="mt-3">
          这个实例没有安装 Iris / Oculus 或 OptiFine，光影包不会生效。可以到「模组管理」里搜索安装 Iris（Fabric / Quilt
          / NeoForge）或 Oculus（Forge）。
        </v-alert>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container">
      <div v-if="loading && packs.length === 0" class="d-flex align-center justify-center pa-10">
        <v-progress-circular indeterminate size="28" color="primary" />
      </div>

      <div v-else-if="error" class="text-center pa-8">
        <div class="text-body-medium text-error mb-3">{{ error }}</div>
        <v-btn size="small" variant="tonal" color="primary" @click="loadPacks">重试</v-btn>
      </div>

      <div v-else-if="packs.length === 0" class="text-center pa-10 text-on-surface-variant">
        <v-icon size="40" class="mb-2" color="on-surface-variant">mdi-shimmer</v-icon>
        <div class="text-body-medium">这个实例还没有光影包</div>
        <div class="text-body-small mt-1">点「安装光影包」从 Modrinth 搜索，或把 zip 拖进 shaderpacks 目录</div>
      </div>

      <v-list v-else lines="two" density="compact">
        <v-list-item v-for="pack in packs" :key="pack.fileName">
          <template #prepend>
            <v-avatar size="40" rounded="lg" class="mr-3" color="surface-container-high">
              <v-icon size="20" :color="pack.selected && shadersEnabled ? 'primary' : 'on-surface-variant'">
                {{ pack.selected && shadersEnabled ? 'mdi-shimmer' : 'mdi-weather-sunset' }}
              </v-icon>
            </v-avatar>
          </template>

          <v-list-item-title
            class="text-body-medium"
            :class="{ 'text-on-surface-variant': !(pack.selected && shadersEnabled) }"
          >
            {{ pack.displayName }}
            <v-chip v-if="pack.selected && shadersEnabled" size="x-small" color="primary" variant="tonal" class="ml-2">
              使用中
            </v-chip>
          </v-list-item-title>
          <v-list-item-subtitle class="text-body-small">
            {{ formatFileSize(pack.size) }} · {{ pack.fileName }}
            <template v-if="pack.isDirectory"> · 文件夹</template>
          </v-list-item-subtitle>

          <template #append>
            <div class="d-flex align-center ga-1">
              <v-chip v-if="!pack.valid" size="x-small" color="error" variant="tonal"> 缺少 shaders </v-chip>
              <v-chip v-if="updateMap.get(pack.fileName)" size="x-small" color="warning" variant="tonal">
                {{ updateMap.get(pack.fileName)?.versionNumber }}
              </v-chip>
              <v-btn
                v-if="updateMap.get(pack.fileName)"
                icon
                size="small"
                variant="text"
                color="primary"
                :loading="updatingFile === pack.fileName"
                :disabled="updatingAll"
                :aria-label="`更新 ${pack.displayName}`"
                @click="updatePack(pack)"
              >
                <v-icon size="18">mdi-update</v-icon>
              </v-btn>
              <v-btn
                size="small"
                variant="tonal"
                color="primary"
                :loading="busyFile === pack.fileName"
                :disabled="pack.selected || busyFile !== null || backend === 'unknown'"
                :aria-label="`启用 ${pack.displayName}`"
                @click="selectPack(pack)"
              >
                {{ pack.selected ? '已选择' : '启用' }}
              </v-btn>
              <v-btn
                icon
                size="small"
                variant="text"
                :aria-label="`删除 ${pack.displayName}`"
                :disabled="busyFile === pack.fileName"
                @click="askDeletePack(pack)"
              >
                <v-icon size="18">mdi-delete-outline</v-icon>
              </v-btn>
            </div>
          </template>
        </v-list-item>
      </v-list>

      <v-divider v-if="packs.length > 0" />
      <div
        v-if="packs.length > 0"
        class="d-flex align-center flex-wrap ga-2 text-body-small text-on-surface-variant pa-3"
      >
        <span>共 {{ packs.length }} 个光影包</span>
        <template v-if="invalidCount > 0">
          <span>·</span>
          <span class="text-error">{{ invalidCount }} 个缺少 shaders 目录，游戏会拒绝加载</span>
        </template>
        <v-spacer />
        <v-btn size="x-small" variant="text" prepend-icon="mdi-folder-open-outline" @click="openFolder">
          打开文件夹
        </v-btn>
      </div>

      <v-divider v-if="!versionIsolation" />
      <div v-if="!versionIsolation" class="text-body-small text-error pa-3">
        当前关闭了版本隔离，光影包在共享目录里，这里的选择会影响所有实例。
      </div>
    </v-card>

    <ShaderPackBrowserDialog
      v-model="browser"
      :instance-name="instanceName"
      :game-version="gameVersion"
      @installed="onInstalled"
    />

    <v-dialog v-model="deleteDialog" max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-title-large font-weight-bold mb-2">删除光影包</div>
          <div class="text-body-medium text-on-surface-variant">
            确定要删除 <strong class="text-on-surface">{{ packToDelete?.displayName }}</strong> 吗？ 文件
            {{ packToDelete?.fileName }} 会直接从磁盘移除。
          </div>
          <div v-if="packToDelete?.selected" class="text-body-small text-error mt-3">
            它正是当前选中的光影包，删除后配置里的选择会被清空，游戏会退回无光影。
          </div>
          <div v-if="!versionIsolation" class="text-body-small text-error mt-3">
            当前关闭了版本隔离，这个文件在共享 shaderpacks 目录里，删除后所有实例都不再能用它。
          </div>
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" :loading="busyFile === packToDelete?.fileName" @click="deletePack">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.min-width-0 {
  min-width: 0;
}
</style>
