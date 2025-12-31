<template>
  <v-container fluid class="add-instance-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center mb-5">
      <v-avatar size="48" color="primary-container" class="mr-3">
        <v-icon size="24" color="on-primary-container">mdi-plus-circle</v-icon>
      </v-avatar>
      <div>
        <h1 class="text-h6 font-weight-bold">添加新实例</h1>
        <p class="text-body-2 text-on-surface-variant mb-0">创建自定义游戏实例或从网络安装整合包</p>
      </div>
    </div>

    <!-- 安装方式选择 - 使用 Tab 样式 -->
    <v-tabs
      v-model="installType"
      color="primary"
      class="mb-4"
      grow
    >
      <v-tab value="custom">
        <v-icon start size="20">mdi-cog</v-icon>
        自定义安装
      </v-tab>
      <v-tab value="online">
        <v-icon start size="20">mdi-cloud-download</v-icon>
        整合包
      </v-tab>
    </v-tabs>

    <!-- 内容区域 -->
    <v-window v-model="installType">
      <!-- 自定义安装 -->
      <v-window-item value="custom">
        <CustomInstallForm />
      </v-window-item>

      <!-- 从互联网安装 -->
      <v-window-item value="online">
        <!-- 平台选择 -->
        <v-chip-group
          v-model="selectedPlatform"
          mandatory
          selected-class="text-primary"
          class="mb-4"
        >
          <v-chip
            value="modrinth"
            variant="tonal"
            filter
            size="large"
          >
            <v-icon start size="18">mdi-alpha-m-circle</v-icon>
            Modrinth
          </v-chip>
          <v-chip
            value="curseforge"
            variant="tonal"
            disabled
            size="large"
          >
            <v-icon start size="18">mdi-fire</v-icon>
            CurseForge
            <v-chip size="x-small" class="ml-1" color="warning" variant="flat">开发中</v-chip>
          </v-chip>
        </v-chip-group>

        <!-- Modrinth 整合包浏览 -->
        <ModrinthBrowser v-if="selectedPlatform === 'modrinth'" />

        <!-- CurseForge 占位 -->
        <v-alert
          v-else-if="selectedPlatform === 'curseforge'"
          type="info"
          variant="tonal"
        >
          CurseForge 整合包支持正在开发中...
        </v-alert>
      </v-window-item>
    </v-window>
  </v-container>
</template>

<script setup lang="ts">
import { ref, defineAsyncComponent } from "vue";

const CustomInstallForm = defineAsyncComponent(() => import('../components/add-instance/CustomInstallForm.vue'));
const ModrinthBrowser = defineAsyncComponent(() => import('../components/add-instance/ModrinthBrowser.vue'));

const installType = ref("custom");
const selectedPlatform = ref("modrinth");
</script>

<style scoped>
.add-instance-container {
  max-width: 900px;
  margin: 0 auto;
}
</style>
