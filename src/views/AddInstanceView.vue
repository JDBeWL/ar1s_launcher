<template>
  <v-container fluid class="page-container pa-4">
    <v-tabs v-model="installType" color="primary" class="mb-4" grow>
      <v-tab value="custom">
        <v-icon start size="20">mdi-cog</v-icon>
        自定义安装
      </v-tab>
      <v-tab value="online">
        <v-icon start size="20">mdi-cloud-download</v-icon>
        整合包
      </v-tab>
    </v-tabs>

    <v-window v-model="installType">
      <v-window-item value="custom">
        <CustomInstallForm />
      </v-window-item>

      <v-window-item value="online">
        <v-chip-group v-model="selectedPlatform" mandatory selected-class="text-primary" class="mb-4">
          <v-chip value="modrinth" variant="tonal" filter size="large">
            <v-icon start size="18">mdi-alpha-m-circle</v-icon>
            Modrinth
          </v-chip>
          <v-chip value="curseforge" variant="tonal" disabled size="large">
            <v-icon start size="18">mdi-fire</v-icon>
            CurseForge
            <v-chip size="x-small" class="ml-1" color="warning" variant="flat">开发中</v-chip>
          </v-chip>
        </v-chip-group>

        <ModrinthBrowser v-if="selectedPlatform === 'modrinth'" />

        <v-alert v-else-if="selectedPlatform === 'curseforge'" type="info" variant="tonal">
          CurseForge 整合包支持正在开发中...
        </v-alert>
      </v-window-item>
    </v-window>
  </v-container>
</template>

<script setup lang="ts">
import { ref, defineAsyncComponent } from 'vue';

const CustomInstallForm = defineAsyncComponent(() => import('../components/add-instance/CustomInstallForm.vue'));
const ModrinthBrowser = defineAsyncComponent(() => import('../components/add-instance/ModrinthBrowser.vue'));

const installType = ref('custom');
const selectedPlatform = ref('modrinth');
</script>
