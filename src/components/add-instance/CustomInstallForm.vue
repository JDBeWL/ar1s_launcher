<script setup lang="ts">
import { onMounted } from 'vue';
import { useInstanceCreation } from '../../composables/useInstanceCreation';

const {
  versions,
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

const versionTypes = [
  { title: "正式版", value: "release" },
  { title: "快照版", value: "snapshot" },
  { title: "全部", value: "all" },
];

const sortOptions = [
  { title: "最新优先", value: "newest" },
  { title: "最旧优先", value: "oldest" },
  { title: "A-Z", value: "az" },
  { title: "Z-A", value: "za" },
];

onMounted(() => {
  fetchVersions();
});
</script>

<template>
  <div>
    <v-row>
      <v-col cols="12">
        <v-text-field
          v-model="instanceName"
          label="实例名称"
          :placeholder="defaultInstanceName"
          hide-details
        ></v-text-field>
      </v-col>
    </v-row>

    <!-- 搜索游戏版本 -->
    <v-row no-gutters class="align-center mt-4 mb-4">
      <v-col class="flex-grow-1 pr-2">
        <v-text-field
          v-model="searchVersion"
          label="搜索版本"
          prepend-inner-icon="mdi-magnify"
          clearable
          hide-details
        ></v-text-field>
      </v-col>
      <v-col class="shrink pr-2" style="max-width: 150px">
        <v-select
          v-model="versionTypeFilter"
          label="版本类型"
          :items="versionTypes"
          hide-details
        ></v-select>
      </v-col>
      <v-col class="shrink" style="max-width: 180px">
        <v-select
          v-model="sortOrder"
          label="排序方式"
          :items="sortOptions"
          hide-details
        ></v-select>
      </v-col>
    </v-row>

    <!-- 游戏版本和Mod加载器选择 -->
    <v-row no-gutters class="align-center mb-4">
      <v-col class="shrink pr-2" style="max-width: 200px">
        <v-select
          v-model="selectedVersion"
          :items="filteredVersions"
          item-title="id"
          item-value="id"
          label="游戏版本"
          :loading="loadingVersions"
          hide-details
          return-object
          @update:model-value="fetchModLoaderVersions"
        ></v-select>
      </v-col>
      <v-col class="shrink pr-2" style="max-width: 200px">
        <v-select
          v-model="selectedModLoaderType"
          :items="modLoaderTypes"
          label="Mod加载器"
          :disabled="!selectedVersion"
          hide-details
          @update:model-value="fetchModLoaderVersions"
        ></v-select>
      </v-col>
      <v-col class="shrink" style="max-width: 1000px">
        <v-select
          v-model="selectedModLoaderVersion"
          :items="modLoaderVersions"
          item-title="version"
          item-value="version"
          label="Mod加载器版本"
          :loading="loadingModLoaderVersions"
          :disabled="!selectedModLoaderType || selectedModLoaderType === 'None'"
          placeholder="请先选择Mod加载器"
          hide-details
          return-object
        ></v-select>
      </v-col>
    </v-row>

    <!-- 进度条 -->
    <v-row v-if="showProgress" class="mt-4">
      <v-col cols="12">
        <div class="d-flex align-center justify-space-between mb-2">
          <span class="text-caption">{{ progressText }}</span>
          <span class="text-caption font-weight-medium">{{ progressValue }}%</span>
        </div>
        <v-progress-linear
          v-model="progressValue"
          color="primary"
          height="8"
          :indeterminate="progressIndeterminate"
        ></v-progress-linear>
      </v-col>
    </v-row>

    <!-- 开始安装按钮 -->
    <v-row class="mt-4">
      <v-col cols="12" class="text-right">
        <v-btn
          color="primary"
          size="large"
          @click="createInstance"
          :disabled="!selectedVersion || installing"
          :loading="installing"
        >
          开始安装
        </v-btn>
      </v-col>
    </v-row>
  </div>
</template>
