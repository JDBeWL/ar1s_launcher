<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { openUrl } from '@tauri-apps/plugin-opener';
import { useAuthStore } from '../../stores/authStore';
import { useDebounceFn } from '../../composables/useDebounce';
import { logError } from '../../utils/logger';

const authStore = useAuthStore()
const loginLoading = ref(false)
const refreshLoading = ref(false)
const loginDialog = ref(false)
const userCode = ref('')
const verificationUri = ref('')
const loginError = ref('')

const debouncedSaveUsername = useDebounceFn((name: string) => {
  authStore.saveUsername(name)
}, 500)

watch(() => authStore.username, (newName) => {
  if (newName !== null && newName !== undefined) {
    debouncedSaveUsername.call(newName)
  }
})

async function switchAuthType(type: 'offline' | 'microsoft') {
  await authStore.switchAuthType(type)
}

async function startMicrosoftLogin() {
  loginLoading.value = true
  loginError.value = ''
  try {
    const display = await authStore.requestDeviceCode()
    userCode.value = display.userCode
    verificationUri.value = display.verificationUri
    loginDialog.value = true

    try {
      await openUrl(display.verificationUri)
    } catch {
      logError('Failed to auto-open browser', undefined, 'AuthSettings')
    }

    try {
      await authStore.completeMicrosoftLogin()
      loginDialog.value = false
    } catch (err) {
      loginDialog.value = false
      const msg = err instanceof Error ? err.message : String(err)
      if (!msg.includes('拒绝了授权') && !msg.includes('已过期')) {
        loginError.value = msg
      }
      logError('Microsoft login failed', err, 'AuthSettings')
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    loginError.value = msg
    logError('Failed to request device code', err, 'AuthSettings')
  } finally {
    loginLoading.value = false
  }
}

async function logoutMicrosoft() {
  await authStore.logoutMicrosoft()
}

async function refreshMicrosoftAuth() {
  refreshLoading.value = true
  try {
    await authStore.tryRefreshMicrosoftToken()
  } finally {
    refreshLoading.value = false
  }
}

function copyUserCode() {
  if (userCode.value) {
    window.navigator.clipboard.writeText(userCode.value)
  }
}

function copyVerificationUri() {
  if (verificationUri.value) {
    window.navigator.clipboard.writeText(verificationUri.value)
  }
}

onMounted(async () => {
  await authStore.init()
})
</script>

<template>
  <div class="settings-group">
    <div class="group-header mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-account-circle-outline</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">账户与认证</h2>
          <p class="text-body-2 text-on-surface-variant mb-0">配置游戏登录方式</p>
        </div>
      </div>
    </div>

    <!-- 认证模式选择 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-shield-check-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">认证模式</span>
        </div>

        <v-btn-toggle
          :model-value="authStore.authType"
          mandatory
          density="comfortable"
          divided
          color="primary"
          variant="outlined"
          class="auth-toggle"
          @update:model-value="switchAuthType($event as 'offline' | 'microsoft')"
        >
          <v-btn value="offline">
            <v-icon start size="18">mdi-wifi-off</v-icon>
            离线模式
          </v-btn>
          <v-btn value="microsoft">
            <v-icon start size="18">mdi-microsoft</v-icon>
            正版登录
          </v-btn>
        </v-btn-toggle>

        <p class="text-caption text-on-surface-variant mt-2 mb-0">
          离线模式无需验证，正版登录使用 Microsoft 账户
        </p>
      </v-card-text>
    </v-card>

    <!-- 离线模式设置 -->
    <v-card v-if="authStore.authType === 'offline'" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-account-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">玩家名称</span>
        </div>
        <v-text-field
          v-model="authStore.username"
          label="玩家名称"
          density="comfortable"
          variant="outlined"
          placeholder="输入游戏名称"
          autocomplete="off"
          hide-details
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-account</v-icon>
          </template>
        </v-text-field>
        <div class="text-caption text-on-surface-variant mt-2">
          离线模式下此名称将作为游戏内的玩家名
        </div>
      </v-card-text>
    </v-card>

    <!-- Microsoft 账户设置 -->
    <v-card v-if="authStore.authType === 'microsoft'" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-microsoft</v-icon>
          <span class="text-subtitle-1 font-weight-medium">Microsoft 账户</span>
        </div>

        <div v-if="authStore.msLoggedIn">
          <v-card color="surface-container-high" variant="flat" class="login-status-card">
            <v-card-text class="pa-3">
              <div class="d-flex align-center">
                <v-avatar size="40" color="primary" class="mr-3">
                  <v-icon size="20" color="on-primary">mdi-account-check</v-icon>
                </v-avatar>
                <div class="flex-grow-1">
                  <div class="text-body-1 font-weight-medium">{{ authStore.msUsername }}</div>
                  <div class="text-caption text-on-surface-variant">正版账户已登录</div>
                </div>
                <v-btn
                  size="small"
                  variant="outlined"
                  color="error"
                  @click="logoutMicrosoft"
                >
                  <v-icon start size="16">mdi-logout</v-icon>
                  登出
                </v-btn>
                <v-btn
                  size="small"
                  variant="outlined"
                  color="primary"
                  class="ml-2"
                  :loading="refreshLoading"
                  @click="refreshMicrosoftAuth"
                >
                  <v-icon start size="16">mdi-refresh</v-icon>
                  刷新
                </v-btn>
              </div>
            </v-card-text>
          </v-card>
        </div>
        <div v-else>
          <v-btn
            block
            variant="outlined"
            color="primary"
            :loading="loginLoading"
            @click="startMicrosoftLogin"
          >
            <v-icon start size="20">mdi-microsoft</v-icon>
            使用 Microsoft 账户登录
          </v-btn>
          <div v-if="loginError" class="text-caption text-error mt-2">
            {{ loginError }}
          </div>
          <div class="text-caption text-on-surface-variant mt-2">
            登录后可使用正版皮肤和在线多人游戏
          </div>
        </div>
      </v-card-text>
    </v-card>

    <!-- Microsoft 登录对话框 -->
    <v-dialog v-model="loginDialog" persistent max-width="420">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-center mb-4">
            <v-avatar size="56" color="primary" class="mb-3">
              <v-icon size="28" color="on-primary">mdi-microsoft</v-icon>
            </v-avatar>
            <div class="text-h6 font-weight-bold">登录 Microsoft 账户</div>
          </div>

          <v-card color="surface-container" variant="flat" class="mb-4">
            <v-card-text class="pa-4">
              <div class="text-body-2 text-on-surface-variant mb-2">请按以下步骤操作：</div>
              <ol class="text-body-2 pl-4">
                <li class="mb-1">在浏览器中打开下方链接</li>
                <li class="mb-1">输入下方验证码</li>
                <li>按照提示完成登录</li>
              </ol>
            </v-card-text>
          </v-card>

          <div class="mb-3">
            <div class="text-caption text-on-surface-variant mb-1">验证链接</div>
            <div class="d-flex align-center">
              <v-chip
                :text="verificationUri"
                variant="outlined"
                color="primary"
                class="flex-grow-1"
                @click="copyVerificationUri"
              >
                {{ verificationUri }}
              </v-chip>
            </div>
          </div>

          <div class="mb-4">
            <div class="text-caption text-on-surface-variant mb-1">验证码</div>
            <div class="d-flex align-center">
              <v-chip
                :text="userCode"
                variant="tonal"
                color="primary"
                size="large"
                class="device-code-chip flex-grow-1"
                @click="copyUserCode"
              >
                <v-icon start size="16">mdi-content-copy</v-icon>
                {{ userCode }}
              </v-chip>
            </div>
          </div>

          <div class="text-center">
            <v-progress-circular indeterminate size="32" color="primary" class="mr-2" />
            <span class="text-body-2 text-on-surface-variant">等待登录中...</span>
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.login-status-card {
  border: 1px solid rgb(var(--v-theme-outline-variant));
}

.device-code-chip {
  font-family: monospace;
  font-size: 1.1rem;
  letter-spacing: 2px;
  justify-content: center;
}
</style>
