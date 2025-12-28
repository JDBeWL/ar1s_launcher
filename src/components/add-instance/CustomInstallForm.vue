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
  modLoaderTypes,
  selectedModLoaderType,
  modLoaderVersions,
  loadingModLoaderVersions,
  selectedModLoaderVersion,
  fetchVersions,
  fetchModLoaderVersions,
  createInstance
} = useInstanceCreation();

onMounted(() => {
  fetchVersions();
});
</script>

<template>
  <div>
    <!-- 实例名称 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="18" class="mr-2">mdi-label-outline</v-icon>
          <span class="text-body-2 font-weight-medium">实例名称</span>
        </div>
        <v-text-field
          v-model="instanceName"
          :placeholder="defaultInstanceName || '输入实例名称'"
          variant="outlined"
          density="compact"
          rounded="lg"
          hide-details
        />
      </v-card-text>
    </v-card>

    <!-- 版本选择 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="18" class="mr-2">mdi-minecraft</v-icon>
          <span class="text-body-2 font-weight-medium">游戏版本</span>
        </div>

        <!-- 搜索和筛选 -->
        <v-row dense class="mb-3">
          <v-col cols="12" sm="5">
            <v-text-field
              v-model="searchVersion"
              placeholder="搜索版本..."
              variant="outlined"
              density="compact"
              rounded="lg"
              hide-details
              clearable
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="4">
            <v-btn-toggle
              v-model="versionTypeFilter"
              mandatory
              rounded="lg"
              density="compact"
              variant="outlined"
              divided
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
              variant="outlined"
              density="compact"
              rounded="lg"
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>

        <!-- 版本选择器 -->
        <v-select
          v-model="selectedVersion"
          :items="filteredVersions"
          item-title="id"
          item-value="id"
          placeholder="选择游戏版本"
          variant="outlined"
          density="compact"
          rounded="lg"
          :loading="loadingVersions"
          hide-details
          return-object
          @update:model-value="fetchModLoaderVersions"
        >
          <template #prepend-inner>
            <v-icon size="18">mdi-gamepad-variant</v-icon>
          </template>
          <template #no-data>
            <v-list-item>
              <v-list-item-title class="text-medium-emphasis">
                没有找到版本
              </v-list-item-title>
            </v-list-item>
          </template>
        </v-select>
      </v-card-text>
    </v-card>

    <!-- Mod 加载器 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="18" class="mr-2">mdi-puzzle</v-icon>
          <span class="text-body-2 font-weight-medium">Mod 加载器</span>
          <span class="text-caption text-medium-emphasis ml-2">(可选)</span>
        </div>

        <v-row dense>
          <v-col cols="12" sm="6">
            <v-select
              v-model="selectedModLoaderType"
              :items="modLoaderTypes"
              placeholder="选择加载器类型"
              variant="outlined"
              density="compact"
              rounded="lg"
              :disabled="!selectedVersion"
              hide-details
              @update:model-value="fetchModLoaderVersions"
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-cog</v-icon>
              </template>
            </v-select>
          </v-col>
          <v-col cols="12" sm="6">
            <v-select
              v-model="selectedModLoaderVersion"
              :items="modLoaderVersions"
              placeholder="选择加载器版本"
              variant="outlined"
              density="compact"
              rounded="lg"
              :loading="loadingModLoaderVersions"
              :disabled="!selectedModLoaderType || selectedModLoaderType === 'None'"
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-tag</v-icon>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-medium-emphasis">
                    {{ selectedModLoaderType === 'None' ? '无需选择' : '没有可用版本' }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 进度条 -->
    <v-card v-if="showProgress" variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-2">
          <span class="text-body-2">{{ progressText }}</span>
          <span class="text-body-2 font-weight-medium">{{ progressValue }}%</span>
        </div>
        <v-progress-linear
          :model-value="progressValue"
          height="8"
          rounded
          :indeterminate="progressIndeterminate"
        />
      </v-card-text>
    </v-card>

    <!-- 开始安装按钮 -->
    <div class="d-flex justify-end">
      <v-btn
        variant="flat"
        rounded="lg"
        size="large"
        @click="createInstance"
        :disabled="!selectedVersion || installing"
        :loading="installing"
      >
        <v-icon start size="20">mdi-download</v-icon>
        开始安装
      </v-btn>
    </div>
  </div>
</template>
