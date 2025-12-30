<script setup lang="ts">
import { onMounted } from 'vue';
import { useInstanceCreation } from '../../composables/useInstanceCreation';

const {
  loadingVersions,
  selectedVersion,
  searchVersion,
  versionTypeFilter,
  sortOrder,
  filteredVersions,
  instanceName,
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

function getLoaderVersionText(item: any): string {
  if (item.version) {
    if (item.mcversion) {
      return item.version;
    }
    const stableText = item.stable === true ? ' (稳定)' : item.stable === false ? ' (测试)' : '';
    return item.version + stableText;
  }
  return String(item);
}

onMounted(() => {
  fetchVersions();
});
</script>

<template>
  <div>
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-label-outline</v-icon>
          <span class="text-body-1 font-weight-medium">实例名称</span>
        </div>
        <v-text-field
          v-model="instanceName"
          :placeholder="defaultInstanceName || '输入实例名称'"
          hide-details
        />
      </v-card-text>
    </v-card>

    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-minecraft</v-icon>
          <span class="text-body-1 font-weight-medium">游戏版本</span>
        </div>

        <v-row dense class="mb-3">
          <v-col cols="12" sm="5">
            <v-text-field
              v-model="searchVersion"
              placeholder="搜索版本..."
              hide-details
              clearable
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="4">
            <v-btn-toggle
              v-model="versionTypeFilter"
              mandatory
              density="comfortable"
              divided
              color="primary"
            >
              <v-btn value="release" size="small">正式版</v-btn>
              <v-btn value="snapshot" size="small">快照</v-btn>
              <v-btn value="all" size="small">全部</v-btn>
            </v-btn-toggle>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              v-model="sortOrder"
              :items="[
                { title: '最新', value: 'newest' },
                { title: '最旧', value: 'oldest' }
              ]"
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>

        <v-select
          v-model="selectedVersion"
          :items="filteredVersions"
          item-title="id"
          item-value="id"
          placeholder="选择游戏版本"
          hide-details
          return-object
        >
          <template #prepend-inner>
            <v-icon size="20" color="on-surface-variant">mdi-gamepad-variant</v-icon>
          </template>
          <template #no-data>
            <v-list-item>
              <v-list-item-title class="text-on-surface-variant">
                没有找到版本
              </v-list-item-title>
            </v-list-item>
          </template>
        </v-select>

        <div v-if="loadingVersions" class="d-flex align-center mt-3">
          <v-progress-linear
            indeterminate
            height="4"
            rounded
            color="primary"
            class="flex-grow-1"
          />
          <span class="text-caption text-on-surface-variant ml-3 text-no-wrap">加载版本中...</span>
        </div>
      </v-card-text>
    </v-card>

    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-puzzle</v-icon>
          <span class="text-body-1 font-weight-medium">Mod 加载器</span>
          <v-chip size="x-small" class="ml-2" color="secondary" variant="tonal">可选</v-chip>
          <v-progress-circular
            v-if="loadingAvailableLoaders"
            indeterminate
            size="16"
            width="2"
            class="ml-2"
          />
        </div>

        <v-row dense>
          <v-col cols="12" sm="6">
            <v-select
              v-model="selectedModLoaderType"
              :items="modLoaderTypes"
              item-title="title"
              item-value="value"
              :item-props="(item: any) => ({ disabled: item.disabled })"
              placeholder="选择加载器类型"
              :disabled="!selectedVersion || loadingAvailableLoaders"
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-cog</v-icon>
              </template>
              <template #item="{ item, props }">
                <v-list-item v-bind="props">
                  <template #append v-if="item.raw.disabled">
                    <v-chip size="x-small" color="warning" variant="tonal">不支持</v-chip>
                  </template>
                </v-list-item>
              </template>
            </v-select>
          </v-col>
          <v-col cols="12" sm="6">
            <v-select
              v-model="selectedModLoaderVersion"
              :items="modLoaderVersions"
              :item-title="getLoaderVersionText"
              item-value="version"
              placeholder="选择加载器版本"
              :disabled="!selectedModLoaderType || selectedModLoaderType === 'None'"
              :loading="loadingModLoaderVersions"
              hide-details
              return-object
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-tag</v-icon>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    {{ selectedModLoaderType === 'None' ? '无需选择' : loadingModLoaderVersions ? '加载中...' : '没有可用版本' }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <v-card v-if="showProgress" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-2">
          <span class="text-body-2">{{ progressText }}</span>
          <span class="text-body-2 font-weight-medium">{{ progressValue }}%</span>
        </div>
        <v-progress-linear
          :model-value="progressValue"
          height="8"
          rounded
          color="primary"
          :indeterminate="progressIndeterminate"
        />
      </v-card-text>
    </v-card>

    <div class="d-flex justify-end">
      <v-btn
        variant="flat"
        color="primary"
        size="large"
        :disabled="!selectedVersion || installing"
        :loading="installing"
        @click="createInstance"
      >
        <v-icon start size="22">mdi-download</v-icon>
        开始安装
      </v-btn>
    </div>
  </div>
</template>
