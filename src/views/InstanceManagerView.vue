<template>
  <v-container fluid>
    <v-tabs v-model="tab" grow>
      <v-tab value="create">创建实例</v-tab>
      <v-tab value="manage">实例列表</v-tab>
    </v-tabs>

    <v-window v-model="tab">
      <v-window-item value="create">
        <!-- 使用与ModLoaderInstallView.vue相同的响应式布局 -->
        <v-container class="py-6" fluid>
          <v-row justify="center">
            <v-col cols="12" md="10" lg="8">
              <v-card class="mt-4">
                <v-card-title>创建新实例</v-card-title>
                <v-card-text>
                  <v-row>
                    <v-col cols="12" sm="6">
                      <v-text-field
                        v-model="searchVersion"
                        label="搜索版本"
                        prepend-inner-icon="mdi-magnify"
                        clearable
                      ></v-text-field>
                    </v-col>
                    <v-col cols="12" sm="6">
                      <v-switch
                        v-model="showReleasesOnly"
                        label="只显示正式版"
                        color="primary"
                      ></v-switch>
                    </v-col>
                  </v-row>

                  <v-select
                    v-model="selectedVersion"
                    :items="filteredVersions"
                    item-title="id"
                    item-value="id"
                    label="选择Minecraft版本"
                    :loading="loadingVersions"
                    return-object
                  >
                    <template v-slot:item="{ props, item }">
                      <v-list-item v-bind="props" :subtitle="item.raw.releaseTime"></v-list-item>
                    </template>
                  </v-select>

                  <v-text-field
                    v-model="instanceName"
                    label="实例名称"
                    :placeholder="defaultInstanceName"
                  ></v-text-field>

                  <v-select
                    v-model="selectedModLoaderType"
                    :items="modLoaderTypes"
                    label="Mod加载器类型"
                    :disabled="!selectedVersion"
                  ></v-select>

                  <v-select
                    v-model="selectedModLoaderVersion"
                    :items="modLoaderVersions"
                    item-title="version"
                    item-value="version"
                    label="Mod加载器版本"
                    :loading="loadingModLoaderVersions"
                    :disabled="!selectedModLoaderType || selectedModLoaderType === 'None'"
                    placeholder="请先选择Mod加载器类型"
                    return-object
                  ></v-select>

                </v-card-text>
                <v-card-actions>
                  <v-spacer></v-spacer>
                  <v-btn
                    color="primary"
                    @click="createInstance"
                    :disabled="!selectedVersion"
                  >
                    创建实例
                  </v-btn>
                </v-card-actions>
              </v-card>
            </v-col>
          </v-row>
        </v-container>
      </v-window-item>

      <v-window-item value="manage">
        <v-container>
           <v-row justify="center">
            <v-col cols="12" md="10" lg="8">
              <v-card class="mt-4">
                <v-card-title>实例列表</v-card-title>
                <v-card-text>
                  <p class="text-center">这里将显示已创建的实例列表。</p>
                  <p class="text-center text-grey">此功能正在开发中。</p>
                  <!-- Placeholder for instance list -->
                </v-card-text>
              </v-card>
            </v-col>
          </v-row>
        </v-container>
      </v-window-item>
    </v-window>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const tab = ref('create');

interface MinecraftVersion {
  id: string;
  type: string;
  url: string;
  time: string;
  releaseTime: string;
}

const versions = ref<MinecraftVersion[]>([]);
const loadingVersions = ref(false);
const selectedVersion = ref<MinecraftVersion | null>(null);
const searchVersion = ref('');
const showReleasesOnly = ref(true);

const instanceName = ref('');

const modLoaderTypes = ['None', 'Forge', 'Fabric', 'Quilt'];
const selectedModLoaderType = ref('None');

const modLoaderVersions = ref<string[]>([]);
const loadingModLoaderVersions = ref(false);
const selectedModLoaderVersion = ref<string | null>(null);

const filteredVersions = computed(() => {
  return versions.value.filter(version => {
    const typeMatch = !showReleasesOnly.value || version.type === 'release';
    const searchMatch = !searchVersion.value || version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
    return typeMatch && searchMatch;
  });
});

const defaultInstanceName = computed(() => {
  if (selectedVersion.value) {
    return selectedVersion.value.id;
  }
  return '';
});

async function fetchVersions() {
  loadingVersions.value = true;
  try {
    const manifest = await invoke('get_versions');
    versions.value = (manifest as any).versions.map((v: any) => ({
      ...v,
      releaseTime: new Date(v.releaseTime).toLocaleString(),
    }));
  } catch (error) {
    console.error("Failed to fetch versions:", error);
  } finally {
    loadingVersions.value = false;
  }
}

async function fetchModLoaderVersions() {
  if (!selectedVersion.value || !selectedModLoaderType.value || selectedModLoaderType.value === 'None') {
    modLoaderVersions.value = [];
    return;
  }

  loadingModLoaderVersions.value = true;
  selectedModLoaderVersion.value = null;

  try {
    if (selectedModLoaderType.value === 'Forge') {
      const result = await invoke('get_forge_versions', { minecraftVersion: selectedVersion.value.id });
      modLoaderVersions.value = result as any[];
    } else {
      // Placeholder for other loaders like Fabric
      modLoaderVersions.value = [];
    }

    if (modLoaderVersions.value.length > 0) {
      selectedModLoaderVersion.value = modLoaderVersions.value[0];
    }

  } catch (error) {
    console.error(`Failed to fetch ${selectedModLoaderType.value} versions:`, error);
    modLoaderVersions.value = []; // Clear on error
  } finally {
    loadingModLoaderVersions.value = false;
  }
}

async function createInstance() {
  if (!selectedVersion.value) {
    alert('请先选择一个Minecraft版本');
    return;
  }

  const finalInstanceName = instanceName.value || defaultInstanceName.value;
  if (!finalInstanceName) {
    alert('实例名称不能为空');
    return;
  }

  try {
    let payload: any = {
      newInstanceName: finalInstanceName,
      baseVersionId: selectedVersion.value.id,
    };

    if (selectedModLoaderType.value === 'Forge' && selectedModLoaderVersion.value) {
      payload.forgeVersion = selectedModLoaderVersion.value;
    }

    await invoke('create_instance', payload);

    alert(`实例 '${finalInstanceName}' 创建成功!`);
    // Optionally, switch to the instance list tab and refresh
    // tab.value = 'manage';
    // fetchInstances(); // This function would need to be created

  } catch (error) {
    console.error("Failed to create instance:", error);
    alert(`创建实例失败: ${error}`);
  }
}

onMounted(() => {
  fetchVersions();
});

watch(selectedVersion, () => {
  selectedModLoaderType.value = 'None';
  selectedModLoaderVersion.value = null;
  modLoaderVersions.value = [];
});

watch(selectedModLoaderType, () => {
  selectedModLoaderVersion.value = null;
  fetchModLoaderVersions();
});

</script>

<style scoped>
.v-card {
  transition: all 0.2s ease-in-out;
}
.v-card:hover {
  box-shadow: 0 8px 16px rgba(0,0,0,0.15);
}
</style>
