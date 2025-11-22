<script setup lang="ts">
defineProps<{
  username: string
  offlineMode: boolean
}>()

defineEmits<{
  (e: 'update:username', value: string): void
  (e: 'update:offlineMode', value: boolean): void
}>()
</script>

<template>
  <v-card>
    <v-card-title>
      <v-icon start>mdi-cog</v-icon>
      游戏设置
    </v-card-title>
    <v-card-text>
      <v-row>
        <v-col cols="12" sm="6">
          <v-text-field
            :model-value="username"
            @update:model-value="$emit('update:username', $event)"
            label="用户名"
            hide-details
          ></v-text-field>
        </v-col>
        <v-col cols="12" sm="6">
          <v-switch
            :model-value="offlineMode"
            @update:model-value="$event !== null && $emit('update:offlineMode', $event)"
            label="离线模式"
            color="primary"
            hide-details
          ></v-switch>
        </v-col>
      </v-row>
    </v-card-text>
  </v-card>
</template>
