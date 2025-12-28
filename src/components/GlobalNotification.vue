<script setup lang="ts">
import { computed } from 'vue'
import { useNotificationStore } from '../stores/notificationStore'

const store = useNotificationStore()

const currentNotification = computed(() => store.notifications[0])
const hasNotification = computed(() => store.notifications.length > 0)

const snackbarColor = computed(() => {
  if (!currentNotification.value) return 'info'
  return currentNotification.value.type
})

const snackbarIcon = computed(() => {
  if (!currentNotification.value) return 'mdi-information'
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[currentNotification.value.type]
})

function closeSnackbar() {
  if (currentNotification.value) {
    store.removeNotification(currentNotification.value.id)
  }
}

const dialogIcon = computed(() => {
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[store.dialogType]
})

const dialogColor = computed(() => store.dialogType)

const confirmIcon = computed(() => {
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[store.confirmType]
})

const confirmColor = computed(() => store.confirmType)
</script>

<template>
  <!-- Snackbar 通知 -->
  <v-snackbar
    :model-value="hasNotification"
    :color="snackbarColor"
    :timeout="-1"
    location="top right"
    elevation="2"
  >
    <div class="d-flex align-center">
      <v-icon class="mr-2">{{ snackbarIcon }}</v-icon>
      <div>
        <div class="font-weight-medium">{{ currentNotification?.title }}</div>
        <div v-if="currentNotification?.message" class="text-caption">
          {{ currentNotification.message }}
        </div>
      </div>
    </div>
    <template #actions>
      <v-btn variant="text" @click="closeSnackbar">关闭</v-btn>
    </template>
  </v-snackbar>

  <!-- 详情 Dialog -->
  <v-dialog v-model="store.dialogVisible" max-width="520">
    <v-card>
      <v-card-title :class="`text-${dialogColor}`">
        <v-icon start :color="dialogColor">{{ dialogIcon }}</v-icon>
        {{ store.dialogTitle }}
      </v-card-title>
      <v-card-text>
        <pre class="dialog-content">{{ store.dialogContent }}</pre>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" @click="store.closeDialog">确定</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <!-- 确认 Dialog -->
  <v-dialog v-model="store.confirmVisible" max-width="420" persistent>
    <v-card>
      <v-card-title :class="`text-${confirmColor}`">
        <v-icon start :color="confirmColor">{{ confirmIcon }}</v-icon>
        {{ store.confirmTitle }}
      </v-card-title>
      <v-card-text>
        {{ store.confirmContent }}
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="store.handleConfirm(false)">取消</v-btn>
        <v-btn color="primary" variant="elevated" @click="store.handleConfirm(true)">确定</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.dialog-content {
  white-space: pre-wrap;
  word-wrap: break-word;
  font-family: inherit;
  margin: 0;
}
</style>
