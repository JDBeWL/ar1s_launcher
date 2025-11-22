<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  instance: any;
}>();

const emit = defineEmits<{
  (e: 'launch', instance: any): void;
  (e: 'open-folder', instance: any): void;
  (e: 'delete', instance: any): void;
  (e: 'rename', instance: any): void;
}>();

const instanceImage = computed(() => {
  // 这里可以根据实例类型或名称返回不同的图片
  // 暂时使用默认图片
  return 'https://cdn.vuetifyjs.com/images/cards/house.jpg'; 
});

function formatLastPlayed(time: number) {
  if (!time) return '从未运行';
  return new Date(time).toLocaleString();
}
</script>

<template>
  <v-card class="instance-card h-100">
    <v-img
      :src="instanceImage"
      height="150"
      cover
      class="align-end"
    >
      <v-card-title class="text-white bg-black-transparent">
        {{ instance.name }}
      </v-card-title>
    </v-img>

    <v-card-subtitle class="pt-4">
      {{ instance.loader_type !== 'None' ? instance.loader_type : '' }} {{ instance.game_version }}
    </v-card-subtitle>

    <v-card-text>
      <div>最后运行: {{ formatLastPlayed(instance.last_played) }}</div>
    </v-card-text>

    <v-card-actions>
      <v-btn
        color="primary"
        variant="elevated"
        prepend-icon="mdi-play"
        @click="emit('launch', instance)"
      >
        启动
      </v-btn>
      
      <v-spacer></v-spacer>

      <v-menu>
        <template v-slot:activator="{ props }">
          <v-btn icon="mdi-dots-vertical" variant="text" v-bind="props"></v-btn>
        </template>
        <v-list>
          <v-list-item @click="emit('open-folder', instance)" prepend-icon="mdi-folder-open">
            <v-list-item-title>打开文件夹</v-list-item-title>
          </v-list-item>
          <v-list-item @click="emit('rename', instance)" prepend-icon="mdi-pencil">
            <v-list-item-title>重命名</v-list-item-title>
          </v-list-item>
          <v-divider></v-divider>
          <v-list-item @click="emit('delete', instance)" prepend-icon="mdi-delete" color="error">
            <v-list-item-title class="text-error">删除实例</v-list-item-title>
          </v-list-item>
        </v-list>
      </v-menu>
    </v-card-actions>
  </v-card>
</template>

<style scoped>
.bg-black-transparent {
  background-color: rgba(0, 0, 0, 0.6);
}

.instance-card {
  transition: transform 0.2s;
}

.instance-card:hover {
  transform: translateY(-4px);
}
</style>
