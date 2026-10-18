<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { useSettingsStore } from '../../stores/settings';
import { configApi, javaApi } from '../../services';
import { logError } from '../../utils/logger';

const settingsStore = useSettingsStore();
const javaPath = ref('');
const isJavaPathValid = ref(false);
const loadingJava = ref(false);
const javaVersion = ref('');
const customJavaPaths = ref<string[]>([]);
const inputJavaPath = ref('');

watch(
  javaPath,
  (val) => {
    inputJavaPath.value = val;
  },
  { immediate: true },
);

async function loadJavaPath() {
  try {
    const path = await configApi.loadConfigKey('javaPath');
    javaPath.value = path || '';
    inputJavaPath.value = javaPath.value;
    if (javaPath.value) {
      isJavaPathValid.value = await javaApi.validateJavaPath(javaPath.value);
      if (isJavaPathValid.value) {
        await getJavaVersion();
      }
    }
  } catch (error) {
    logError('Failed to load Java path', error, 'JavaSettings');
  }
}

async function toggleAutoMatchJava(value: boolean | null) {
  settingsStore.autoMatchJava = value === true;
  await settingsStore.saveAutoMatchJava();
}

async function loadCustomJavaPaths() {
  try {
    customJavaPaths.value = await javaApi.getCustomJavaPaths();
    await loadCustomJavaVersions();
  } catch (error) {
    logError('Failed to load custom Java paths', error, 'JavaSettings');
  }
}

async function getJavaVersion() {
  try {
    javaVersion.value = await javaApi.getJavaVersion(javaPath.value);
  } catch {
    javaVersion.value = '';
  }
}

async function findJavaInstallations() {
  try {
    loadingJava.value = true;
    await settingsStore.findJavaInstallations();
    await loadDetectedJavaVersions();

    if (settingsStore.javaInstallations.length > 0 && !javaPath.value) {
      await selectJavaPath(settingsStore.javaInstallations[0]);
    }
  } catch (err) {
    logError('Failed to find Java installations', err, 'JavaSettings');
  } finally {
    loadingJava.value = false;
  }
}

async function selectJavaPath(path: string) {
  try {
    javaPath.value = path;
    inputJavaPath.value = path;
    await configApi.saveConfigKey('javaPath', path);
    isJavaPathValid.value = await javaApi.validateJavaPath(path);
    if (isJavaPathValid.value) {
      await getJavaVersion();
    }
  } catch (err) {
    logError('Failed to set Java path', err, 'JavaSettings');
  }
}

async function browseJavaPath() {
  try {
    const selected = await open({
      multiple: false,
      title: '选择 Java 可执行文件',
      filters: [
        {
          name: 'Java',
          extensions: ['exe', ''],
        },
      ],
    });
    if (selected) {
      await selectJavaPath(selected as string);
    }
  } catch (err) {
    logError('Failed to browse Java path', err, 'JavaSettings');
  }
}

async function commitInputPath() {
  const path = inputJavaPath.value.trim();
  if (!path) return;
  if (path === javaPath.value) return;
  await selectJavaPath(path);
}

function onInputKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault();
    (e.target as HTMLInputElement).blur();
  }
}

async function addCustomJava() {
  try {
    const selected = await open({
      multiple: false,
      title: '选择 Java 可执行文件',
      filters: [
        {
          name: 'Java',
          extensions: ['exe', ''],
        },
      ],
    });
    if (selected) {
      const path = selected as string;
      const isValid = await javaApi.validateJavaPath(path);
      if (!isValid) {
        return;
      }
      await javaApi.addCustomJavaPath(path);
      await loadCustomJavaPaths();
    }
  } catch (err) {
    logError('Failed to add custom Java path', err, 'JavaSettings');
  }
}

async function removeCustomJava(path: string) {
  try {
    await javaApi.removeCustomJavaPath(path);
    await loadCustomJavaPaths();
  } catch (err) {
    logError('Failed to remove custom Java path', err, 'JavaSettings');
  }
}

async function fetchJavaVersion(path: string): Promise<string> {
  try {
    return await javaApi.getJavaVersion(path);
  } catch {
    return '';
  }
}

const customJavaVersions = ref<Record<string, string>>({});

async function loadCustomJavaVersions() {
  const entries = await Promise.all(
    customJavaPaths.value.map(async (path) => {
      const version = await fetchJavaVersion(path);
      return [path, version] as const;
    }),
  );
  customJavaVersions.value = Object.fromEntries(entries);
}

const detectedJavaVersions = ref<Record<string, string>>({});

async function loadDetectedJavaVersions() {
  const entries = await Promise.all(
    settingsStore.javaInstallations.map(async (path) => {
      const version = await fetchJavaVersion(path);
      return [path, version] as const;
    }),
  );
  detectedJavaVersions.value = Object.fromEntries(entries);
}

onMounted(async () => {
  await Promise.all([loadJavaPath(), loadCustomJavaPaths(), settingsStore.loadSettings()]);

  if (!settingsStore.hasFoundJavaInstallations && settingsStore.javaInstallations.length === 0) {
    await findJavaInstallations();
  } else {
    await loadDetectedJavaVersions();
  }
});
</script>

<template>
  <div class="settings-group">
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-file-cog-outline</v-icon>
            <span class="text-title-medium">Java 路径</span>
          </div>
          <div class="d-flex ga-2">
            <v-btn variant="tonal" color="primary" size="small" :loading="loadingJava" @click="findJavaInstallations">
              <v-icon start size="18">mdi-magnify</v-icon>
              自动查找
            </v-btn>
            <v-btn variant="tonal" color="secondary" size="small" @click="browseJavaPath">
              <v-icon start size="18">mdi-folder-open-outline</v-icon>
              浏览
            </v-btn>
          </div>
        </div>

        <v-text-field
          v-model="inputJavaPath"
          density="compact"
          variant="outlined"
          label="Java 路径"
          placeholder="输入 Java 路径或选择下方已检测的安装"
          hide-details
          class="mb-4 font-mono-input"
          @keydown="onInputKeydown"
          @blur="commitInputPath"
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-console</v-icon>
          </template>
          <template #append-inner>
            <v-icon
              v-if="inputJavaPath && inputJavaPath === javaPath"
              :color="isJavaPathValid ? 'success' : 'error'"
              size="18"
            >
              {{ isJavaPathValid ? 'mdi-check-circle' : 'mdi-alert-circle' }}
            </v-icon>
          </template>
        </v-text-field>

        <div v-if="javaPath" class="d-flex align-center mb-4">
          <v-icon size="16" :color="isJavaPathValid ? 'success' : 'error'" class="mr-2">
            {{ isJavaPathValid ? 'mdi-check-circle' : 'mdi-alert-circle' }}
          </v-icon>
          <span v-if="isJavaPathValid" class="text-body-small text-on-surface-variant">
            {{ javaVersion || '路径有效' }}
          </span>
          <span v-else class="text-body-small text-error">路径无效，无法执行</span>
        </div>

        <div v-if="settingsStore.javaInstallations.length > 0" class="mb-2">
          <div class="text-body-medium text-on-surface-variant mb-2">检测到的 Java 安装：</div>
          <v-list density="compact" lines="two" class="java-list" bg-color="surface-container-high">
            <v-list-item
              v-for="path in settingsStore.javaInstallations"
              :key="'detected-' + path"
              :active="javaPath === path"
              @click="selectJavaPath(path)"
            >
              <template #prepend>
                <v-icon :color="javaPath === path ? 'primary' : 'on-surface-variant'">
                  {{ javaPath === path ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank' }}
                </v-icon>
              </template>
              <v-list-item-title class="text-body-medium font-mono">
                {{ path.replace(/\\/g, '/') }}
              </v-list-item-title>
              <v-list-item-subtitle v-if="detectedJavaVersions[path]" class="text-body-small">
                {{ detectedJavaVersions[path] }}
              </v-list-item-subtitle>
            </v-list-item>
          </v-list>
        </div>

        <v-alert v-if="!javaPath" color="warning-container" density="compact">
          <template #prepend>
            <v-icon color="on-warning-container">mdi-alert-outline</v-icon>
          </template>
          <span class="text-on-warning-container">未检测到 Java，请在上方输入路径或点击"自动查找"</span>
        </v-alert>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-bookmark-multiple-outline</v-icon>
            <div>
              <span class="text-title-medium">自定义 Java</span>
              <div class="text-body-small text-on-surface-variant">手动添加常用的 Java 路径，方便快速切换</div>
            </div>
          </div>
          <v-btn variant="tonal" color="primary" size="small" @click="addCustomJava">
            <v-icon start size="18">mdi-plus</v-icon>
            添加
          </v-btn>
        </div>

        <div v-if="customJavaPaths.length > 0">
          <v-list density="compact" lines="two" class="java-list" bg-color="surface-container-high">
            <v-list-item
              v-for="path in customJavaPaths"
              :key="'custom-' + path"
              :active="javaPath === path"
              @click="selectJavaPath(path)"
            >
              <template #prepend>
                <v-icon :color="javaPath === path ? 'primary' : 'on-surface-variant'">
                  {{ javaPath === path ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank' }}
                </v-icon>
              </template>
              <v-list-item-title class="text-body-medium font-mono">
                {{ path.replace(/\\/g, '/') }}
              </v-list-item-title>
              <v-list-item-subtitle v-if="customJavaVersions[path]" class="text-body-small">
                {{ customJavaVersions[path] }}
              </v-list-item-subtitle>
              <template #append>
                <v-btn icon size="x-small" variant="text" color="error" @click.stop="removeCustomJava(path)">
                  <v-icon size="16">mdi-close</v-icon>
                </v-btn>
              </template>
            </v-list-item>
          </v-list>
        </div>
        <div v-else class="text-body-small text-on-surface-variant">
          尚未添加自定义 Java 路径，点击"添加"按钮手动选择
        </div>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-auto-fix</v-icon>
            <div>
              <span class="text-title-medium">自动匹配 Java 版本</span>
              <div class="text-body-small text-on-surface-variant">
                启动游戏时根据 Minecraft 版本自动选择合适的 Java
              </div>
            </div>
          </div>
          <v-switch
            :model-value="settingsStore.autoMatchJava"
            aria-label="自动匹配 Java 版本"
            color="primary"
            hide-details
            inset
            density="compact"
            @update:model-value="toggleAutoMatchJava"
          />
        </div>
        <v-alert v-if="settingsStore.autoMatchJava" color="secondary-container" density="compact" class="mt-3">
          <template #prepend>
            <v-icon color="on-secondary-container" size="18">mdi-information-outline</v-icon>
          </template>
          <span class="text-on-secondary-container text-body-medium">
            启用后，启动时按实例声明的 javaVersion 选 Java（读不到声明时按 MC 版本推断），mods
            的字节码版本会抬高下限；同版本优先 64 位与正式版，选不到声明版本时会说明用的是哪份
          </span>
        </v-alert>
      </v-card-text>
    </v-card>

    <v-alert color="secondary-container" class="tip-alert">
      <template #prepend>
        <v-icon size="20" color="on-secondary-container">mdi-information-outline</v-icon>
      </template>
      <template #title>
        <span class="text-body-medium font-weight-medium text-on-secondary-container">Java 版本建议</span>
      </template>
      <ul class="text-body-small pl-4 mb-0 mt-1 text-on-secondary-container">
        <li>Minecraft 1.17+ 需要 Java 17 或更高版本</li>
        <li>Minecraft 1.16.5 及以下版本建议使用 Java 8</li>
        <li>32 位 Java 最多只能分配约 1.5G 内存，同版本会优先选 64 位</li>
        <li>推荐使用 Adoptium (Eclipse Temurin) 或 Azul Zulu</li>
        <li>支持相对路径（相对于启动器目录），如 <code class="font-mono">./runtime/java/bin/java.exe</code></li>
      </ul>
    </v-alert>
  </div>
</template>

<style scoped>
.java-list {
  max-height: 200px;
  overflow-y: auto;
}

.font-mono {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.875rem;
}

.font-mono-input :deep(input) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.875rem;
}
</style>
