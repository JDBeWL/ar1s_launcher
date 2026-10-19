<script setup lang="ts">
/**
 * 实例详情页
 * 从实例列表进入，按标签组织这个实例的模组、游戏设置、资源包与光影
 */
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import InstanceAvatar from '../components/instance/InstanceAvatar.vue';
import ModsPanel from '../components/instance/ModsPanel.vue';
import ResourcePacksPanel from '../components/instance/ResourcePacksPanel.vue';
import ShaderPacksPanel from '../components/instance/ShaderPacksPanel.vue';
import { useNotificationStore } from '../stores/notificationStore';
import { getInstanceListCache } from '../stores/instanceListCache';
import { instanceApi } from '../services';
import { useGameLaunch } from '../composables/useGameLaunch';
import type { GameInstance } from '../types/events';
import { formatLastPlayed } from '../utils/format';
import { logError } from '../utils/logger';

const route = useRoute();
const router = useRouter();
const notificationStore = useNotificationStore();
const { launchInstance: startInstance } = useGameLaunch();

const tab = ref<'mods' | 'settings' | 'resourcepacks' | 'shaderpacks'>('mods');

const instanceName = computed(() => String(route.params.name ?? ''));

// 列表页已经拉过一次，先用缓存把头部画出来，再静默刷新，避免整页转圈闪一下
const cachedInstance = getInstanceListCache().find((item) => item.name === instanceName.value) ?? null;
const instance = ref<GameInstance | null>(cachedInstance);
const loading = ref(cachedInstance === null);
const error = ref('');
const launching = ref(false);

const versionLabel = computed(() => {
  const value = instance.value;
  if (!value) return '';
  const mc = value.gameVersion || value.version;
  const loader = value.loaderType && value.loaderType !== 'None' ? value.loaderType : '';
  return loader ? `${mc} · ${loader}` : mc;
});

/**
 * silent 用于模组操作后的刷新：不切加载态，避免把整个页面（连同模组面板的更新列表）重建一遍
 */
async function loadInstance(silent = false) {
  if (!silent) {
    loading.value = true;
  }
  error.value = '';
  try {
    const instances = await instanceApi.getInstances();
    instance.value = instances.find((item) => item.name === instanceName.value) ?? null;
    if (!instance.value) {
      error.value = `没有找到名为「${instanceName.value}」的实例，可能已被删除或重命名`;
    }
  } catch (e) {
    logError('Failed to load instance detail', e, 'InstanceDetailView');
    error.value = '加载实例信息失败';
  } finally {
    if (!silent) {
      loading.value = false;
    }
  }
}

async function launch() {
  if (!instance.value) return;

  launching.value = true;
  try {
    if (await startInstance(instance.value.name)) {
      await loadInstance(true);
    }
  } finally {
    launching.value = false;
  }
}

async function openFolder() {
  if (!instance.value) return;
  try {
    await instanceApi.openInstanceFolder(instance.value.name);
  } catch (e) {
    logError('Failed to open instance folder', e, 'InstanceDetailView');
    notificationStore.error('打开文件夹失败', '请检查实例目录是否还存在');
  }
}

function goBack() {
  router.push({ name: 'instance-manager' });
}

watch(instanceName, () => {
  const cached = getInstanceListCache().find((item) => item.name === instanceName.value) ?? null;
  instance.value = cached;
  loading.value = cached === null;
  void loadInstance(cached !== null);
});

onMounted(() => {
  void loadInstance(cachedInstance !== null);
});
</script>

<template>
  <v-container fluid class="page-container pa-4">
    <div class="d-flex align-center mb-3">
      <v-btn variant="text" prepend-icon="mdi-arrow-left" @click="goBack">返回实例列表</v-btn>
    </div>

    <v-card v-if="loading" variant="flat" color="surface-container" class="pa-8">
      <div class="d-flex align-center justify-center">
        <v-progress-circular indeterminate color="primary" />
      </div>
    </v-card>

    <v-card v-else-if="error" variant="flat" color="surface-container" class="pa-8 text-center">
      <v-icon size="40" class="mb-2" color="on-surface-variant">mdi-alert-circle-outline</v-icon>
      <div class="text-body-large mb-4">{{ error }}</div>
      <v-btn color="primary" variant="tonal" @click="goBack">返回实例列表</v-btn>
    </v-card>

    <template v-else-if="instance">
      <v-card variant="flat" color="surface-container" class="mb-3">
        <v-card-text class="pa-4 d-flex align-center flex-wrap ga-3">
          <InstanceAvatar :instance="instance" :size="56" />

          <div class="flex-grow-1 min-width-0">
            <div class="text-title-large font-weight-bold text-truncate">{{ instance.name }}</div>
            <div class="text-body-small text-on-surface-variant">
              {{ versionLabel }}
              <template v-if="instance.modCount"> · {{ instance.modCount }} Mods</template>
              <template v-if="instance.lastPlayed"> · {{ formatLastPlayed(instance.lastPlayed) }}</template>
            </div>
          </div>

          <div class="d-flex align-center ga-2">
            <v-btn variant="text" prepend-icon="mdi-folder-open-outline" @click="openFolder">打开文件夹</v-btn>
            <v-btn color="primary" :loading="launching" @click="launch">
              <v-icon start>mdi-play</v-icon>
              启动
            </v-btn>
          </div>
        </v-card-text>
      </v-card>

      <v-tabs v-model="tab" color="primary" class="mb-3">
        <v-tab value="mods">
          <v-icon start size="18">mdi-puzzle-outline</v-icon>
          模组管理
        </v-tab>
        <v-tab value="settings">
          <v-icon start size="18">mdi-cog-outline</v-icon>
          游戏设置
        </v-tab>
        <v-tab value="resourcepacks">
          <v-icon start size="18">mdi-texture-box</v-icon>
          资源包
        </v-tab>
        <v-tab value="shaderpacks">
          <v-icon start size="18">mdi-shimmer</v-icon>
          光影
        </v-tab>
      </v-tabs>

      <v-window v-model="tab">
        <v-window-item value="mods">
          <ModsPanel
            :instance-name="instance.name"
            :game-version="instance.gameVersion"
            :loader="instance.loaderType"
            @changed="loadInstance(true)"
          />
        </v-window-item>

        <v-window-item value="settings">
          <v-card variant="flat" color="surface-container">
            <v-card-text class="pa-4">
              <div class="text-title-small mb-3">实例信息</div>
              <v-row density="compact">
                <v-col cols="12" sm="6" md="3">
                  <div class="text-body-small text-on-surface-variant">Minecraft 版本</div>
                  <div class="text-body-medium">{{ instance.gameVersion || instance.version }}</div>
                </v-col>
                <v-col cols="12" sm="6" md="3">
                  <div class="text-body-small text-on-surface-variant">加载器</div>
                  <div class="text-body-medium">
                    {{ instance.loaderType && instance.loaderType !== 'None' ? instance.loaderType : '原版' }}
                    <template v-if="instance.loaderVersion"> {{ instance.loaderVersion }}</template>
                  </div>
                </v-col>
                <v-col cols="12" sm="6" md="3">
                  <div class="text-body-small text-on-surface-variant">模组数量</div>
                  <div class="text-body-medium">{{ instance.modCount ?? 0 }}</div>
                </v-col>
                <v-col cols="12" sm="6" md="3">
                  <div class="text-body-small text-on-surface-variant">最后启动</div>
                  <div class="text-body-medium">{{ formatLastPlayed(instance.lastPlayed) }}</div>
                </v-col>
              </v-row>

              <v-divider class="my-4" />

              <v-alert type="info" variant="tonal" density="comfortable">
                实例级的内存、Java 与窗口设置还在开发中。目前这些参数在「设置」里统一调整，启动时按实例的
                版本与模组自动匹配 Java。
              </v-alert>

              <div class="d-flex ga-2 mt-4">
                <v-btn variant="tonal" prepend-icon="mdi-folder-open-outline" @click="openFolder">
                  打开实例文件夹
                </v-btn>
                <v-btn variant="text" prepend-icon="mdi-cog-outline" to="/settings">打开全局设置</v-btn>
              </div>
            </v-card-text>
          </v-card>
        </v-window-item>

        <v-window-item value="resourcepacks">
          <ResourcePacksPanel
            :instance-name="instance.name"
            :game-version="instance.gameVersion"
            @changed="loadInstance(true)"
          />
        </v-window-item>

        <v-window-item value="shaderpacks">
          <ShaderPacksPanel
            :instance-name="instance.name"
            :game-version="instance.gameVersion"
            @changed="loadInstance(true)"
          />
        </v-window-item>
      </v-window>
    </template>
  </v-container>
</template>

<style scoped>
.min-width-0 {
  min-width: 0;
}
</style>
