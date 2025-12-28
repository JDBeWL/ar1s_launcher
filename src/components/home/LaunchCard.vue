<script setup lang="ts">
import type { DownloadProgress } from '../../types/events';

defineProps<{
  selectedVersion: string
  loading: boolean
  isRepairing?: boolean
  repairProgress?: DownloadProgress | null
}>()

defineEmits<{
  (e: 'launch'): void
}>()
</script>

<template>
  <v-card class="mb-4">
    <v-card-title class="text-center">
      启动游戏
    </v-card-title>
    <v-card-text class="text-center">
      <v-btn
        block
        color="primary"
        size="large"
        :loading="loading"
        :disabled="!selectedVersion"
        @click="$emit('launch')"
        class="mb-2"
      >
        <v-icon start>mdi-play</v-icon>
        启动 Minecraft
      </v-btn>
      
      <div class="text-caption text-medium-emphasis">
        <v-icon size="16" class="mr-1">mdi-information</v-icon>
        {{ !selectedVersion ? '请先选择游戏版本' : '准备就绪' }}
      </div>
    </v-card-text>

    <!-- Repair Progress Dialog -->
    <v-dialog :model-value="isRepairing" persistent max-width="400">
      <v-card>
        <v-card-title>正在修复游戏文件</v-card-title>
        <v-card-text>
          <div v-if="repairProgress">
            <div class="d-flex justify-space-between mb-2">
              <span>{{ repairProgress.status === 'downloading' ? '下载中...' : '处理中...' }}</span>
              <span>{{ repairProgress.percent }}%</span>
            </div>
            <v-progress-linear
              :model-value="repairProgress.percent"
              color="primary"
              height="20"
              striped
            ></v-progress-linear>
            <div class="d-flex justify-space-between mt-2 text-caption text-medium-emphasis">
              <span>{{ (repairProgress.progress / 1024 / 1024).toFixed(2) }} MB / {{ (repairProgress.total / 1024 / 1024).toFixed(2) }} MB</span>
              <span>{{ repairProgress.speed.toFixed(2) }} KB/s</span>
            </div>
          </div>
          <div v-else class="text-center pa-4">
            <v-progress-circular indeterminate color="primary"></v-progress-circular>
            <div class="mt-2">准备中...</div>
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>
  </v-card>
</template>
