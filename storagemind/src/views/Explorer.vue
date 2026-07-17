<template>
  <div class="card flex-col" style="height: 100%;">
    <div class="card-header">
      <div class="card-title">File Explorer</div>
      <div class="card-subtitle">Browse your indexed files</div>
    </div>
    
    <!-- Path Breadcrumbs -->
    <div class="flex items-center gap-2 mb-4 text-tertiary font-mono font-sm p-2" style="background: var(--color-bg-overlay); border-radius: var(--radius-sm);">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
        <polyline points="9 22 9 12 15 12 15 22"></polyline>
      </svg>
      <span>/ Root</span>
    </div>

    <!-- File List -->
    <div style="flex: 1; overflow-y: auto;">
      <table class="data-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Size</th>
            <th>Modified</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="file in files" :key="file.id">
            <td class="flex items-center gap-2">
              <svg v-if="file.kind === 'directory'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="text-accent">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
              </svg>
              <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="text-tertiary">
                <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
                <polyline points="13 2 13 9 20 9"></polyline>
              </svg>
              <span>{{ file.name }}</span>
            </td>
            <td><span :class="['badge', 'badge-' + file.category]">{{ file.category }}</span></td>
            <td class="font-mono text-tertiary">{{ file.kind === 'directory' ? '--' : formatSize(file.size) }}</td>
            <td class="text-tertiary font-sm">{{ formatDate(file.modifiedAt) }}</td>
          </tr>
        </tbody>
      </table>
      
      <div v-if="files.length === 0" class="empty-state">
        <div class="empty-state-icon">📁</div>
        <div class="empty-state-title">No files found</div>
        <div class="empty-state-desc">This directory is empty or hasn't been scanned yet.</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'

const files = ref<any[]>([])
// const currentParentId = ref<number | null>(null)

onMounted(async () => {
  await loadChildren(null) // Root
})

const loadChildren = async (parentId: number | null) => {
  try {
    const data: any = await invoke('cmd_list_children', { 
      params: { parentId, limit: 100, offset: 0 } 
    })
    files.value = data
  } catch (e) {
    console.error('Failed to load files:', e)
  }
}

const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const formatDate = (timestamp: number | null) => {
  if (!timestamp) return '--'
  return dayjs(timestamp * 1000).format('YYYY-MM-DD HH:mm')
}
</script>
