<script setup lang="ts">
import { computed } from 'vue'
import { useLauncherStore } from '../stores/launcherStore'

const store = useLauncherStore()

const snackVisible = computed({
  get: () => store.gameSnackVisible,
  set: (v: boolean) => { store.gameSnackVisible = v }
})
const snackText = computed(() => store.gameSnackText)
const snackColor = computed(() => store.gameSnackColor)

const dialogVisible = computed({
  get: () => store.gameDialogVisible,
  set: (v: boolean) => { store.gameDialogVisible = v }
})
const dialogTitle = computed(() => store.gameDialogTitle)
const dialogText = computed(() => store.gameDialogText)
</script>

<template>
  <!-- 全局 Snackbar：提示游戏退出或错误摘要 -->
  <v-snackbar
    v-model="snackVisible"
    :color="snackColor"
    timeout="4000"
    location="top right"
    elevation="2"
  >
    {{ snackText }}
    <template #actions>
      <v-btn variant="text" @click="snackVisible = false">
        关闭
      </v-btn>
    </template>
  </v-snackbar>

  <!-- 全局 Dialog：显示错误详情 -->
  <v-dialog v-model="dialogVisible" max-width="520">
    <v-card>
      <v-card-title class="text-h6">
        {{ dialogTitle }}
      </v-card-title>
      <v-card-text>
        <pre style="white-space: pre-wrap; word-wrap: break-word; font-family: inherit;">{{ dialogText }}</pre>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" @click="dialogVisible = false">确定</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>