<template>
  <v-container>
    <v-card>
      <v-card-title class="d-flex mt-2 align-center"> 添加新实例 </v-card-title>
      <v-card-text>
        <!-- 安装方式选择 -->
        <v-row class="mb-4">
          <v-col cols="12" class="d-flex align-center">
            <div class="d-flex align-center" style="width: 100%">
              <div
                class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                :class="{ 'install-type-active': installType === 'custom' }"
                @click="installType = 'custom'"
              >
                自定义安装
              </div>
              <div class="install-type-divider"></div>
              <div
                class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                :class="{ 'install-type-active': installType === 'online' }"
                @click="installType = 'online'"
              >
                从互联网安装
              </div>
            </div>
          </v-col>
        </v-row>

        <!-- 自定义安装内容 -->
        <div v-if="installType === 'custom'">
          <CustomInstallForm />
        </div>

        <!-- 从互联网安装内容 -->
        <div v-if="installType === 'online'">
          <!-- 平台选择 -->
          <v-row class="mb-4">
            <v-col cols="12">
              <div class="d-flex">
                <v-btn
                  :color="selectedPlatform === 'modrinth' ? 'primary' : 'grey lighten-3'"
                  :class="selectedPlatform === 'modrinth' ? 'elevation-4' : 'elevation-1'"
                  height="60"
                  width="200"
                  @click="selectedPlatform = 'modrinth'"
                  class="platform-btn mr-4"
                >
                  <span class="text-h6 font-weight-bold">Modrinth</span>
                </v-btn>
                <v-btn
                  :color="selectedPlatform === 'curseforge' ? 'primary' : 'grey lighten-3'"
                  :class="selectedPlatform === 'curseforge' ? 'elevation-4' : 'elevation-1'"
                  height="60"
                  width="200"
                  @click="selectedPlatform = 'curseforge'"
                  class="platform-btn"
                  disabled
                >
                  <span class="text-h6 font-weight-bold">CurseForge</span>
                  <v-chip small color="orange" class="ml-2">开发中</v-chip>
                </v-btn>
              </div>
            </v-col>
          </v-row>

          <!-- Modrinth整合包搜索 -->
          <div v-if="selectedPlatform === 'modrinth'">
            <ModrinthBrowser />
          </div>

          <!-- CurseForge整合包搜索 (占位) -->
          <div v-if="selectedPlatform === 'curseforge'">
            <v-alert type="info" class="mb-4">
              CurseForge整合包支持正在开发中...
            </v-alert>
          </div>
        </div>
      </v-card-text>
    </v-card>
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
.install-type-tab {
  border-bottom: 2px solid transparent;
  transition: all 0.3s ease;
  font-size: 1.1rem;
  font-weight: 500;
  color: grey;
}

.install-type-active {
  border-bottom-color: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-primary));
  font-weight: bold;
}

.install-type-divider {
  width: 1px;
  height: 24px;
  background-color: #e0e0e0;
  margin: 0 16px;
}

.platform-btn {
  transition: all 0.3s ease;
  border-radius: 12px;
}

/* 确保子组件样式正确应用 */
:deep(.v-card-title) {
  font-size: 1.25rem;
  font-weight: 600;
}
</style>
