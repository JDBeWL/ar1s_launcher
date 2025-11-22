<script setup lang="ts">
defineProps<{
  selectedVersion: string
  installedVersions: string[]
  loading: boolean
}>()

defineEmits<{
  (e: 'update:selectedVersion', value: string): void
  (e: 'refresh'): void
}>()
</script>

<template>
  <v-card class="mb-4">
    <v-card-title class="d-flex align-center">
      <v-icon start>mdi-play-circle</v-icon>
      版本选择
      <v-spacer></v-spacer>
      <v-btn 
        variant="text" 
        icon="mdi-refresh" 
        @click="$emit('refresh')" 
        :loading="loading"
      ></v-btn>
    </v-card-title>
    <v-card-text>
      <v-select
        :model-value="selectedVersion"
        @update:model-value="$emit('update:selectedVersion', $event)"
        :items="installedVersions"
        label="选择已安装的游戏版本"
        :loading="loading"
        :hint="installedVersions.length === 0 ? '没有找到已安装的版本，请先在下载页面下载' : ''"
        persistent-hint
      ></v-select>

      <div class="d-flex justify-end">
        <v-btn
          variant="text"
          color="primary"
          prepend-icon="mdi-download"
          to="/download"
          size="small"
        >
          下载新版本
        </v-btn>
      </div>
    </v-card-text>
  </v-card>
</template>
