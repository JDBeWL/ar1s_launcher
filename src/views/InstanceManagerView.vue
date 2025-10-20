<template>
  <v-container fluid>
    <!-- 标题卡片 -->
    <v-card class="mb-6" elevation="2">
      <v-card-title class="d-flex mt-2 justify-space-between align-center">
        实例管理
        <v-btn color="primary" variant="outlined" @click="fetchInstances" :loading="loading">
          <v-icon start>mdi-refresh</v-icon>
          刷新列表
        </v-btn>
      </v-card-title>
      
      <!-- 加载状态 -->
      <v-row v-if="loading" class="pa-4">
        <v-col cols="12" class="text-center">
          <v-progress-circular indeterminate color="primary"></v-progress-circular>
          <div class="mt-2">正在加载实例列表...</div>
        </v-col>
      </v-row>
      
      <!-- 空状态 -->
      <v-row v-else-if="instances.length === 0" class="pa-4">
        <v-col cols="12" class="text-center">
          <v-icon size="64" color="grey">mdi-folder-open-outline</v-icon>
          <div class="mt-2 text-grey">暂无实例，请先创建实例</div>
        </v-col>
      </v-row>
      
      <!-- 实例卡片网格布局 -->
      <v-row v-else class="pa-4">
        <v-col v-for="instance in instances" :key="instance.id" cols="12" sm="6" md="4" lg="3">
          <v-card class="instance-card" elevation="2">
            <div class="card-top">
              <!-- 左边图标占位符 -->
              <div class="icon-placeholder"></div>
              
              <!-- 右边信息区域 -->
              <div class="info-area">
                <div class="instance-name">{{ instance.name }}</div>
                <div class="version-info">{{ instance.version }}</div>
                <div v-if="instance.created_time" class="created-time">
                  {{ formatTime(instance.created_time) }}
                </div>
              </div>
            </div>
            <div class="card-bottom">
              <v-btn class="action-btn" icon variant="text" size="small" @click="editInstance(instance)" title="重命名">
                <v-icon>mdi-pencil</v-icon>
              </v-btn>
              
              <v-btn class="action-btn" icon variant="text" size="small" @click="openInstanceFolder(instance)" title="打开文件夹">
                <v-icon>mdi-folder-open</v-icon>
              </v-btn>
              
              <v-btn class="action-btn" icon variant="text" size="small" @click="deleteInstance(instance)" title="删除" color="error">
                <v-icon>mdi-delete</v-icon>
              </v-btn>
              
              <v-btn class="action-btn" icon variant="text" size="small" @click="launchInstance(instance)" title="启动" color="success">
                <v-icon>mdi-play</v-icon>
              </v-btn>
            </div>
          </v-card>
        </v-col>
      </v-row>
    </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useLauncherStore } from '../stores/launcherStore';

interface Instance {
  id: string;
  name: string;
  version: string;
  path: string;
  created_time?: string;
}

const instances = ref<Instance[]>([]);
const loading = ref(false);
const store = useLauncherStore();

// 获取实例列表
async function fetchInstances() {
  loading.value = true;
  try {
    const realInstances = await invoke<Instance[]>('get_instances');
    instances.value = realInstances;
  } catch (error) {
    console.error('获取实例列表失败:', error);
    store.gameSnackText = '获取实例列表失败';
    store.gameSnackColor = 'error';
    store.gameSnackVisible = true;
  } finally {
    loading.value = false;
  }
}

// 编辑实例名称
async function editInstance(instance: Instance) {
  const newName = prompt(`请输入新的实例名称:`, instance.name);
  if (newName && newName.trim() && newName !== instance.name) {
    try {
      await invoke('rename_instance', { 
        oldName: instance.name, 
        newName: newName.trim() 
      });
      store.gameSnackText = '实例重命名成功';
      store.gameSnackColor = 'success';
      store.gameSnackVisible = true;
      await fetchInstances(); // 刷新列表
    } catch (error) {
      console.error('重命名实例失败:', error);
      store.gameSnackText = '重命名实例失败';
      store.gameSnackColor = 'error';
      store.gameSnackVisible = true;
    }
  }
}

// 打开实例文件夹
async function openInstanceFolder(instance: Instance) {
  try {
    await invoke('open_instance_folder', { instanceName: instance.name });
  } catch (error) {
    console.error('打开实例文件夹失败:', error);
    store.gameSnackText = '打开实例文件夹失败';
    store.gameSnackColor = 'error';
    store.gameSnackVisible = true;
  }
}

// 启动实例
async function launchInstance(instance: Instance) {
  try {
    // 这里需要调用启动游戏的API
    store.gameSnackText = `正在启动实例: ${instance.name}`;
    store.gameSnackColor = 'info';
    store.gameSnackVisible = true;
    console.log('启动实例:', instance);
  } catch (error) {
    console.error('启动实例失败:', error);
    store.gameSnackText = '启动实例失败';
    store.gameSnackColor = 'error';
    store.gameSnackVisible = true;
  }
}

// 删除实例
async function deleteInstance(instance: Instance) {
  if (confirm(`确定要删除实例 "${instance.name}" 吗？此操作不可撤销。`)) {
    try {
      await invoke('delete_instance', { instanceName: instance.name });
      store.gameSnackText = '实例删除成功';
      store.gameSnackColor = 'success';
      store.gameSnackVisible = true;
      await fetchInstances(); // 刷新列表
    } catch (error) {
      console.error('删除实例失败:', error);
      store.gameSnackText = '删除实例失败';
      store.gameSnackColor = 'error';
      store.gameSnackVisible = true;
    }
  }
}

// 格式化时间戳
function formatTime(timestamp: string): string {
  const date = new Date(parseInt(timestamp) * 1000);
  return date.toLocaleDateString('zh-CN') + ' ' + date.toLocaleTimeString('zh-CN', { 
    hour: '2-digit', 
    minute: '2-digit' 
  });
}

onMounted(() => {
  fetchInstances();
});
</script>

<style scoped>
.instance-card {
  height: 160px; /* 稍微增加高度以适应更多内容 */
  border-radius: 12px;
  background-color: #f5f5f5; /* 灰色背景 */
  display: flex;
  flex-direction: column;
  transition: all 0.3s ease;
}

.instance-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
}

.card-top {
  flex: 3; /* 增加顶部区域比例 */
  display: flex;
  padding: 12px;
}

.icon-placeholder {
  width: 54px;
  height: 54px;
  background-color: rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  margin-right: 12px;
  margin-top: 10px;
}

.info-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.instance-name {
  font-size: 1.175rem;
  font-weight: 600;
  color: #000;
  line-height: 1.2;
  margin-bottom: 4px;
  word-break: break-word;
}

.version-info {
  font-size: 0.825rem;
  color: #666;
  line-height: 1.2;
  margin-bottom: 2px;
}

.created-time {
  font-size: 0.75rem;
  color: #999;
  line-height: 1.2;
}

.card-bottom {
  flex: 1;
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 8px;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.action-btn {
  width: 32px;
  height: 32px;
  transition: all 0.2s ease;
}

.action-btn:hover {
  transform: scale(1.1);
}

/* 深色模式适配 */
:deep(.v-theme--dark) .instance-card {
  background-color: #424242;
}

:deep(.v-theme--dark) .instance-name {
  color: #fff;
}

:deep(.v-theme--dark) .version-info {
  color: #ccc;
}

:deep(.v-theme--dark) .created-time {
  color: #aaa;
}

:deep(.v-theme--dark) .card-bottom {
  border-top-color: rgba(255, 255, 255, 0.1);
}

:deep(.v-theme--dark) .icon-placeholder {
  background-color: rgba(255, 255, 255, 0.1);
}

/* 响应式调整 */
@media (max-width: 960px) {
  .instance-card {
    height: 140px;
  }
  
  .card-top {
    padding: 8px;
  }
  
  .icon-placeholder {
    width: 48px;
    height: 48px;
    margin-right: 8px;
  }
  
  .instance-name {
    font-size: 1rem;
  }
  
  .version-info, .created-time {
    font-size: 0.75rem;
  }
}
</style>