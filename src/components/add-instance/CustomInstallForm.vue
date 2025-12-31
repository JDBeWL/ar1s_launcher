<script setup lang="ts">
import { onMounted, computed } from 'vue';
import { useInstanceCreation } from '../../composables/useInstanceCreation';
import { useRouter } from 'vue-router';
import { getLoaderSelectIcon } from '../../utils/format';
import type { ForgeVersion, LoaderVersionInfo } from '../../types/events';

const router = useRouter();

const {
  loadingVersions,
  selectedVersion,
  searchVersion,
  versionTypeFilter,
  filteredVersions,
  instanceName,
  instanceNameError,
  defaultInstanceName,
  installing,
  showProgress,
  progressValue,
  progressIndeterminate,
  progressText,
  loadingAvailableLoaders,
  modLoaderTypes,
  selectedModLoaderType,
  modLoaderVersions,
  loadingModLoaderVersions,
  selectedModLoaderVersion,
  fetchVersions,
  createInstance
} = useInstanceCreation();

// 类型守卫：检查是否为 LoaderVersionInfo（有 stable 属性）
function hasStableProperty(item: ForgeVersion | LoaderVersionInfo): item is LoaderVersionInfo {
  return 'stable' in item;
}

// 计算实际使用的实例名称
const effectiveInstanceName = computed(() => {
  return instanceName.value.trim() || defaultInstanceName.value;
});

// 计算是否可以创建
const canCreate = computed(() => {
  return selectedVersion.value && 
         !installing.value && 
         !instanceNameError.value &&
         // 如果选择了加载器，必须选择版本
         (selectedModLoaderType.value === 'None' || selectedModLoaderVersion.value);
});

// 获取加载器版本显示文本
function getLoaderVersionText(item: any): string {
  if (!item) return '';
  if (item.version) {
    if (item.mcversion) {
      return item.version;
    }
    const stableText = item.stable === true ? ' (稳定)' : item.stable === false ? ' (测试)' : '';
    return item.version + stableText;
  }
  return String(item);
}

// 创建实例后跳转
async function handleCreate() {
  await createInstance();
  if (!installing.value && !showProgress.value) {
    router.push('/instance-manager');
  }
}

onMounted(() => {
  fetchVersions();
});
</script>

<template>
  <div>
    <!-- 游戏版本选择 -->
    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="primary">mdi-minecraft</v-icon>
            <span class="text-body-1 font-weight-medium">游戏版本</span>
          </div>
          <v-btn-toggle
            v-model="versionTypeFilter"
            mandatory
            density="compact"
            divided
            color="primary"
            variant="outlined"
          >
            <v-btn value="release" size="small">正式版</v-btn>
            <v-btn value="snapshot" size="small">快照</v-btn>
            <v-btn value="all" size="small">全部</v-btn>
          </v-btn-toggle>
        </div>

        <v-row dense>
          <v-col cols="12" sm="8">
            <v-select
              v-model="selectedVersion"
              :items="filteredVersions"
              item-title="id"
              item-value="id"
              placeholder="选择 Minecraft 版本"
              density="comfortable"
              variant="outlined"
              hide-details
              return-object
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-gamepad-variant</v-icon>
              </template>
              <template #item="{ item, props }">
                <v-list-item v-bind="props">
                  <template #append>
                    <v-chip 
                      size="x-small" 
                      :color="item.raw.type === 'release' ? 'success' : 'warning'"
                      variant="tonal"
                    >
                      {{ item.raw.type === 'release' ? '正式版' : '快照' }}
                    </v-chip>
                  </template>
                </v-list-item>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    {{ loadingVersions ? '加载中...' : '没有找到版本' }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </v-col>
          <v-col cols="12" sm="4">
            <v-text-field
              v-model="searchVersion"
              placeholder="搜索版本..."
              density="comfortable"
              variant="outlined"
              hide-details
              clearable
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- Mod 加载器选择 -->
    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="primary">mdi-puzzle</v-icon>
          <span class="text-body-1 font-weight-medium">Mod 加载器</span>
          <v-chip size="x-small" class="ml-2" color="secondary" variant="tonal">可选</v-chip>
          <v-spacer />
          <v-fade-transition>
            <v-progress-circular
              v-if="loadingAvailableLoaders"
              indeterminate
              size="16"
              width="2"
              color="primary"
            />
          </v-fade-transition>
        </div>

        <!-- 加载器类型选择 - 使用 Chip 组 -->
        <div class="loader-chips mb-4">
          <v-chip
            v-for="loader in modLoaderTypes"
            :key="loader.value"
            :variant="selectedModLoaderType === loader.value ? 'flat' : 'tonal'"
            :color="selectedModLoaderType === loader.value ? 'primary' : undefined"
            :disabled="loader.disabled || !selectedVersion || loadingAvailableLoaders"
            class="mr-2 mb-2"
            @click="selectedModLoaderType = loader.value"
          >
            <v-icon start size="16">{{ getLoaderSelectIcon(loader.value) }}</v-icon>
            {{ loader.title }}
            <v-tooltip v-if="loader.disabled && selectedVersion" activator="parent" location="top">
              该版本不支持 {{ loader.title }}
            </v-tooltip>
          </v-chip>
        </div>

        <!-- 加载器版本选择 -->
        <v-expand-transition>
          <div v-if="selectedModLoaderType !== 'None'">
            <v-select
              v-model="selectedModLoaderVersion"
              :items="modLoaderVersions"
              :item-title="getLoaderVersionText"
              item-value="version"
              :placeholder="`选择 ${selectedModLoaderType} 版本`"
              density="comfortable"
              variant="outlined"
              :disabled="loadingModLoaderVersions"
              hide-details
              return-object
            >
              <template #prepend-inner>
                <v-progress-circular
                  v-if="loadingModLoaderVersions"
                  indeterminate
                  size="16"
                  width="2"
                  color="primary"
                  class="mr-1"
                />
                <v-icon v-else size="18" color="on-surface-variant">mdi-tag</v-icon>
              </template>
              <template #item="{ item, props }">
                <v-list-item v-bind="props">
                  <template #append>
                    <v-chip 
                      v-if="hasStableProperty(item.raw) && item.raw.stable === true" 
                      size="x-small" 
                      color="success" 
                      variant="tonal"
                    >
                      稳定
                    </v-chip>
                    <v-chip 
                      v-else-if="hasStableProperty(item.raw) && item.raw.stable === false" 
                      size="x-small" 
                      color="warning" 
                      variant="tonal"
                    >
                      测试
                    </v-chip>
                  </template>
                </v-list-item>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    {{ loadingModLoaderVersions ? '加载中...' : '没有可用版本' }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </div>
        </v-expand-transition>

        <!-- 未选择版本提示 -->
        <v-alert
          v-if="!selectedVersion"
          type="info"
          variant="tonal"
          density="compact"
          class="mt-2"
        >
          请先选择游戏版本以查看可用的加载器
        </v-alert>
      </v-card-text>
    </v-card>

    <!-- 实例名称 -->
    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="primary">mdi-rename</v-icon>
          <span class="text-body-1 font-weight-medium">实例名称</span>
        </div>
        
        <v-text-field
          v-model="instanceName"
          :placeholder="defaultInstanceName || '输入实例名称'"
          :error-messages="instanceNameError || undefined"
          density="comfortable"
          variant="outlined"
          hide-details="auto"
          counter="64"
          maxlength="64"
        >
          <template #append-inner>
            <v-fade-transition>
              <v-icon 
                v-if="effectiveInstanceName && !instanceNameError" 
                size="18" 
                color="success"
              >
                mdi-check-circle
              </v-icon>
            </v-fade-transition>
          </template>
        </v-text-field>
        
        <div class="text-caption text-on-surface-variant mt-2">
          留空将使用 <span class="font-weight-medium">{{ defaultInstanceName || '版本号' }}</span> 作为实例名称
        </div>
      </v-card-text>
    </v-card>

    <!-- 安装进度 -->
    <v-expand-transition>
      <v-card v-if="showProgress" color="surface-container-high" variant="flat" class="mb-4">
        <v-card-text class="pa-4">
          <div class="d-flex align-center justify-space-between mb-2">
            <div class="d-flex align-center">
              <v-progress-circular
                v-if="progressIndeterminate"
                indeterminate
                size="18"
                width="2"
                color="primary"
                class="mr-2"
              />
              <v-icon v-else size="18" color="primary" class="mr-2">mdi-download</v-icon>
              <span class="text-body-2">{{ progressText }}</span>
            </div>
            <span class="text-body-2 font-weight-medium text-primary">{{ progressValue }}%</span>
          </div>
          <v-progress-linear
            :model-value="progressValue"
            :indeterminate="progressIndeterminate"
            height="6"
            rounded
            color="primary"
            bg-color="surface-container"
          />
        </v-card-text>
      </v-card>
    </v-expand-transition>

    <!-- 配置预览和操作按钮 -->
    <v-card v-if="selectedVersion && !showProgress" color="primary-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between">
          <div>
            <div class="text-body-2 text-on-primary-container opacity-70">即将创建</div>
            <div class="text-body-1 font-weight-medium text-on-primary-container">
              {{ effectiveInstanceName }}
            </div>
            <div class="d-flex align-center ga-1 mt-1">
              <v-chip size="x-small" color="on-primary-container" variant="outlined">
                {{ selectedVersion.id }}
              </v-chip>
              <v-chip 
                v-if="selectedModLoaderType !== 'None' && selectedModLoaderVersion" 
                size="x-small" 
                color="on-primary-container" 
                variant="outlined"
              >
                {{ selectedModLoaderType }} {{ (selectedModLoaderVersion as any).version }}
              </v-chip>
            </div>
          </div>
          <v-btn
            color="on-primary-container"
            variant="flat"
            :disabled="!canCreate"
            :loading="installing"
            @click="handleCreate"
          >
            <v-icon start>mdi-download</v-icon>
            创建实例
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <!-- 未选择版本时的按钮 -->
    <div v-else-if="!showProgress" class="d-flex justify-end">
      <v-btn
        color="primary"
        size="large"
        disabled
      >
        <v-icon start>mdi-download</v-icon>
        创建实例
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.loader-chips {
  display: flex;
  flex-wrap: wrap;
}
</style>
