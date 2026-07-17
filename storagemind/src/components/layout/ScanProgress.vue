<template>
  <div v-if="isVisible" class="scan-progress-overlay">
    <div class="progress-header flex justify-between items-center">
      <div class="flex items-center gap-2">
        <div class="spinner-small"></div>
        <span class="font-bold">Scanning...</span>
      </div>
      <button class="btn-icon" @click="isVisible = false">
        <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2" fill="none">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
    
    <div class="progress-details mt-2">
      <div class="text-secondary font-sm truncate" :title="currentPath" style="direction: rtl; text-align: left;">
        &hellip;{{ currentPath.slice(-40) }}
      </div>
      
      <div class="stats-row mt-2 flex justify-between font-sm text-tertiary">
        <span>Files: {{ formatNumber(filesScanned) }}</span>
        <span>Dirs: {{ formatNumber(dirsScanned) }}</span>
        <span>Size: {{ formatSize(bytesScanned) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'

const isVisible = ref(false)
const currentPath = ref('')
const filesScanned = ref(0)
const dirsScanned = ref(0)
const bytesScanned = ref(0)
let unlistenProgress: any = null
let unlistenStarted: any = null
let unlistenCompleted: any = null

onMounted(async () => {
  unlistenStarted = await listen('scan:started', () => {
    isVisible.value = true
    filesScanned.value = 0
    dirsScanned.value = 0
    bytesScanned.value = 0
    currentPath.value = 'Initializing...'
  })

  unlistenProgress = await listen('scan:progress', (event: any) => {
    isVisible.value = true
    const data = event.payload.Progress
    if (data) {
      filesScanned.value = data.files_scanned
      dirsScanned.value = data.dirs_scanned
      bytesScanned.value = data.bytes_scanned
      currentPath.value = data.current_path
    }
  })

  unlistenCompleted = await listen('scan:completed', () => {
    currentPath.value = 'Scan completed'
    setTimeout(() => {
      isVisible.value = false
    }, 3000)
  })
})

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress()
  if (unlistenStarted) unlistenStarted()
  if (unlistenCompleted) unlistenCompleted()
})

const formatNumber = (num: number) => new Intl.NumberFormat().format(num)
const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
</script>

<style scoped>
.scan-progress-overlay {
  position: fixed;
  bottom: 24px;
  right: 24px;
  width: 320px;
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 16px;
  box-shadow: 0 10px 25px rgba(0,0,0,0.5);
  z-index: 9999;
  animation: slideUp 0.3s ease-out forwards;
}

@keyframes slideUp {
  from { transform: translateY(20px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

.spinner-small {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255,255,255,0.2);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.truncate {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
