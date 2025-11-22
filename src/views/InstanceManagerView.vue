<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import InstanceCard from "../components/instance/InstanceCard.vue";

const router = useRouter();
const instances = ref<any[]>([]);
const loading = ref(false);

// 重命名对话框
const renameDialog = ref(false);
const renameInstanceName = ref("");
const currentInstance = ref<any>(null);

// 删除确认对话框
const deleteDialog = ref(false);

async function loadInstances() {
  loading.value = true;
  try {
    const result = await invoke("get_instances");
    instances.value = result as any[];
  } catch (error) {
    console.error("Failed to load instances:", error);
  } finally {
    loading.value = false;
  }
}

function launchInstance(instance: any) {
  // 跳转到主页并选中该实例，或者直接启动
  // 这里简单起见，我们跳转到主页，实际逻辑可能需要通过Store传递选中的实例
  router.push({ path: "/", query: { instance: instance.name } });
}

async function openInstanceFolder(instance: any) {
  try {
    await invoke("open_instance_folder", { instanceName: instance.name });
  } catch (error) {
    console.error("Failed to open folder:", error);
  }
}

function openRenameDialog(instance: any) {
  currentInstance.value = instance;
  renameInstanceName.value = instance.name;
  renameDialog.value = true;
}

async function renameInstance() {
  if (!currentInstance.value || !renameInstanceName.value) return;
  
  try {
    await invoke("rename_instance", { 
      oldName: currentInstance.value.name, 
      newName: renameInstanceName.value 
    });
    renameDialog.value = false;
    await loadInstances();
  } catch (error) {
    console.error("Failed to rename instance:", error);
    alert(`重命名失败: ${error}`);
  }
}

function openDeleteDialog(instance: any) {
  currentInstance.value = instance;
  deleteDialog.value = true;
}

async function deleteInstance() {
  if (!currentInstance.value) return;
  
  try {
    await invoke("delete_instance", { instanceName: currentInstance.value.name });
    deleteDialog.value = false;
    await loadInstances();
  } catch (error) {
    console.error("Failed to delete instance:", error);
    alert(`删除失败: ${error}`);
  }
}

onMounted(() => {
  loadInstances();
});
</script>

<template>
  <v-container>
    <v-card>
      <v-card-title class="d-flex mt-2 align-center justify-space-between">
        <span>实例管理</span>
        <v-btn
          color="primary"
          prepend-icon="mdi-plus"
          to="/add-instance"
          variant="elevated"
        >
          新建实例
        </v-btn>
      </v-card-title>

      <v-card-text>
        <!-- 加载状态 -->
        <v-row v-if="loading">
          <v-col cols="12" class="text-center py-8">
            <v-progress-circular 
              indeterminate 
              color="primary"
              size="64"
            ></v-progress-circular>
            <div class="mt-4 text-h6">正在加载实例...</div>
          </v-col>
        </v-row>

        <!-- 空状态 -->
        <v-row v-else-if="instances.length === 0">
          <v-col cols="12" class="text-center py-12">
            <v-icon size="96" color="grey-lighten-1">mdi-cube-outline</v-icon>
            <div class="text-h5 text-grey mt-4">没有找到实例</div>
            <div class="text-body-1 text-grey-darken-1 mt-2">
              创建您的第一个 Minecraft 实例来开始游戏
            </div>
            <v-btn 
              color="primary" 
              class="mt-6" 
              to="/add-instance"
              size="large"
              prepend-icon="mdi-plus"
            >
              创建新实例
            </v-btn>
          </v-col>
        </v-row>

        <!-- 实例列表 -->
        <v-row v-else>
          <v-col
            v-for="instance in instances"
            :key="instance.name"
            cols="12"
            sm="6"
            md="4"
            lg="3"
          >
            <InstanceCard
              :instance="instance"
              @launch="launchInstance"
              @open-folder="openInstanceFolder"
              @rename="openRenameDialog"
              @delete="openDeleteDialog"
            />
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 重命名对话框 -->
    <v-dialog v-model="renameDialog" max-width="400">
      <v-card>
        <v-card-title>重命名实例</v-card-title>
        <v-card-text>
          <v-text-field
            v-model="renameInstanceName"
            label="新名称"
            autofocus
            variant="outlined"
            density="comfortable"
          ></v-text-field>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="grey" variant="text" @click="renameDialog = false">取消</v-btn>
          <v-btn color="primary" variant="elevated" @click="renameInstance">确定</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认对话框 -->
    <v-dialog v-model="deleteDialog" max-width="400">
      <v-card>
        <v-card-title class="text-error">
          <v-icon start>mdi-alert-circle</v-icon>
          删除实例
        </v-card-title>
        <v-card-text>
          <div class="text-body-1">
            确定要删除实例 <strong>"{{ currentInstance?.name }}"</strong> 吗？
          </div>
          <div class="text-body-2 text-grey-darken-1 mt-2">
            此操作无法撤销，所有数据将被永久删除。
          </div>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn color="grey" variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" variant="elevated" @click="deleteInstance">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>
</template>