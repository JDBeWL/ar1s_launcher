<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps({
  version: {
    type: String,
    required: true
  },
  progress: {
    type: Number,
    default: 0
  },
  total: {
    type: Number,
    default: 0
  },
  speed: {
    type: Number,
    default: 0
  },
  status: {
    type: String,
    default: 'idle'
  }
})

const progressPercentage = computed(() => {
  if (props.total === 0) return 0
  return (props.progress / props.total) * 100
})

const formattedSpeed = computed(() => {
  if (props.speed < 1024) return `${props.speed.toFixed(2)} KB/s`
  return `${(props.speed / 1024).toFixed(2)} MB/s`
})

const remainingTime = computed(() => {
  if (props.speed === 0 || props.progress === 0) return '--'
  const remainingBytes = props.total - props.progress
  const seconds = remainingBytes / (props.speed * 1024)
  
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)
  
  return `${hours ? `${hours}h ` : ''}${minutes ? `${minutes}m ` : ''}${secs}s`
})
</script>

<template>
  <v-card class="notification-download" width="400">
    <v-card-title class="d-flex align-center">
      <v-icon class="mr-2" color="primary">mdi-download</v-icon>
      <span class="text-subtitle-1">{{ version }}</span>
      <v-spacer></v-spacer>
      <v-btn icon size="small" variant="text">
        <v-icon>mdi-close</v-icon>
      </v-btn>
    </v-card-title>
    
    <v-card-text>
      <v-progress-linear
        :model-value="progressPercentage"
        height="8"
        color="primary"
        class="mb-2"
      ></v-progress-linear>
      
      <div class="d-flex justify-space-between text-caption">
        <span>{{ progress }} / {{ total }} 文件</span>
        <span>{{ progressPercentage.toFixed(1) }}%</span>
      </div>
      
      <div class="d-flex justify-space-between text-caption mt-1">
        <span>{{ formattedSpeed }}</span>
        <span>剩余: {{ remainingTime }}</span>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.notification-download {
  position: fixed;
  top: 75px;
  right: 8px;
  z-index: 1000;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}
</style>