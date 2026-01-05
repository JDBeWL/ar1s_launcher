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
  },
  modelValue: {
    type: Boolean,
    default: true
  }
})

const emit = defineEmits(['update:modelValue', 'cancel'])

const progressPercentage = computed(() => {
  if (props.total === 0) return 0
  const percentage = (props.progress / props.total) * 100
  return Math.min(percentage, 100) // 限制最大值为 100%
})

const formattedSpeed = computed(() => {
  if (props.speed < 1024) return `${props.speed.toFixed(2)} KB/s`
  return `${(props.speed / 1024).toFixed(2)} MB/s`
})

const remainingTime = computed(() => {
  // 速度为 0 时无法计算剩余时间
  if (props.speed === 0) return '--'
  // 还没有下载数据或已完成时不显示剩余时间
  if (props.total === 0) return '--'
  
  const remainingBytes = props.total - props.progress
  if (remainingBytes <= 0) return '0s'
  
  const seconds = remainingBytes / (props.speed * 1024)
  
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)
  
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m ${secs}s`
  return `${secs}s`
})

const progressText = computed(() => {
  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };
  return `${formatBytes(props.progress)} / ${formatBytes(props.total)}`;
})

// 处理关闭通知
function hideNotification() {
  emit('update:modelValue', false)
}

// 处理取消下载
function handleCancel() {
  emit('cancel')
}
</script>

<template>
  <v-card v-if="modelValue" class="notification-download" width="400">
    <v-card-title class="d-flex align-center">
      <v-icon class="mr-2" color="primary">mdi-download</v-icon>
      <span class="text-subtitle-1">{{ version }}</span>
      <v-spacer></v-spacer>
      <v-btn v-if="status === 'downloading'" icon size="small" variant="text" @click="handleCancel" title="取消下载">
        <v-icon>mdi-cancel</v-icon>
      </v-btn>
      <v-btn icon size="small" variant="text" @click="hideNotification" title="隐藏通知">
        <v-icon>mdi-close</v-icon>
      </v-btn>
    </v-card-title>
    
    <v-card-text>
      <v-progress-linear
        :model-value="progressPercentage"
        height="8"
        :color="status === 'completed' ? 'success' : status === 'error' ? 'error' : 'primary'"
        class="mb-2"
      ></v-progress-linear>
      
      <div class="d-flex justify-space-between text-caption">
        <span>{{ progressText }}</span>
        <span>{{ progressPercentage.toFixed(1) }}%</span>
      </div>
      
      <div class="d-flex justify-space-between text-caption mt-1">
        <span>{{ formattedSpeed }}</span>
        <span>剩余: {{ remainingTime }}</span>
      </div>
      
      <div v-if="status === 'completed'" class="text-success text-caption mt-1">
        下载完成
      </div>
      <div v-else-if="status === 'error'" class="text-error text-caption mt-1">
        下载失败
      </div>
      <div v-else-if="status === 'cancelled'" class="text-warning text-caption mt-1">
        下载已取消
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