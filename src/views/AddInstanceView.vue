<template>
  <v-container fluid class="add-instance-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center mb-4">
      <v-avatar size="48" color="primary-container" class="mr-3">
        <v-icon size="24" color="on-primary-container">mdi-plus-circle</v-icon>
      </v-avatar>
      <div>
        <h1 class="text-h6 font-weight-bold">添加新实例</h1>
        <p class="text-body-2 text-on-surface-variant mb-0">创建自定义游戏实例或从网络安装整合包</p>
      </div>
    </div>

    <!-- 安装方式选择 -->
    <v-card color="surface-container" rounded="lg" class="mb-4">
      <v-card-text class="pa-2">
        <v-btn-toggle
          v-model="installType"
          mandatory
          rounded="lg"
          density="compact"
          divided
          class="w-100"
        >
          <v-btn value="custom" class="flex-grow-1">
            <v-icon start size="18">mdi-cog</v-icon>
            自定义安装
          </v-btn>
          <v-btn value="online" class="flex-grow-1">
            <v-icon start size="18">mdi-cloud-download</v-icon>
            从互联网安装
          </v-btn>
        </v-btn-toggle>
      </v-card-text>
    </v-card>

    <!-- 自定义安装内容 -->
    <div v-if="installType === 'custom'">
      <CustomInstallForm />
    </div>

    <!-- 从互联网安装内容 -->
    <div v-if="installType === 'online'">
      <!-- 平台选择 -->
      <v-card color="surface-container" rounded="lg" class="mb-4">
        <v-card-text class="pa-3">
          <div class="text-body-2 text-medium-emphasis mb-2">选择平台</div>
          <div class="d-flex ga-2">
            <v-btn
              :variant="selectedPlatform === 'modrinth' ? 'flat' : 'tonal'"
              rounded="lg"
              @click="selectedPlatform = 'modrinth'"
              class="platform-btn"
            >
              <v-icon start size="18">mdi-alpha-m-circle</v-icon>
              Modrinth
            </v-btn>
            <v-btn
              variant="tonal"
              rounded="lg"
              disabled
              class="platform-btn"
            >
              <v-icon start size="18">mdi-fire</v-icon>
              CurseForge
              <v-chip size="x-small" class="ml-2" variant="tonal">开发中</v-chip>
            </v-btn>
          </div>
        </v-card-text>
      </v-card>

      <!-- Modrinth整合包搜索 -->
      <div v-if="selectedPlatform === 'modrinth'">
        <ModrinthBrowser />
      </div>

      <!-- CurseForge整合包搜索 (占位) -->
      <div v-if="selectedPlatform === 'curseforge'">
        <v-alert variant="tonal" rounded="lg">
          <template #prepend>
            <v-icon>mdi-information-outline</v-icon>
          </template>
          CurseForge 整合包支持正在开发中...
        </v-alert>
      </div>
    </div>
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

.platform-btn {
  min-width: 140px;
}
</style>
