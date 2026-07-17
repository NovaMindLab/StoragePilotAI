<template>
  <div class="dashboard-page flex-col gap-6" style="display: flex;">
    
    <!-- Big Scan Center Banner -->
    <div class="scan-banner card flex-col" style="background: linear-gradient(135deg, rgba(79, 142, 247, 0.1) 0%, rgba(155, 109, 255, 0.1) 100%); border: 1px solid rgba(155, 109, 255, 0.2); position: relative; overflow: hidden;">
      <!-- decorative background circles -->
      <div style="position: absolute; right: -50px; top: -50px; width: 200px; height: 200px; background: var(--color-primary); filter: blur(80px); opacity: 0.2; border-radius: 50%;"></div>
      <div style="position: absolute; left: 20%; bottom: -30px; width: 150px; height: 150px; background: var(--color-accent-purple); filter: blur(60px); opacity: 0.15; border-radius: 50%;"></div>
      
      <div class="flex items-center justify-between" style="position: relative; z-index: 1;">
        <div class="flex-col gap-2">
          <h2 class="text-xl font-bold" style="color: #fff; display: flex; items-center; gap: 8px;">
            <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
              <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
            </svg>
            AI Engine Scanner
          </h2>
          <div class="text-secondary font-sm max-w-lg">
            Unleash the power of MobileCLIP to semantically index your files. 
            Choose a directory and let the AI extract rich metadata and vector embeddings.
          </div>
        </div>
        
        <div class="flex-col items-end gap-2" style="min-width: 300px;">
          <div v-if="!isScanning" class="flex gap-2 w-full">
            <input type="text" class="search-input" style="flex: 1; background: rgba(0,0,0,0.2);" v-model="scanPath" placeholder="Directory path (e.g. /Users/hou/Pictures)" />
            <button class="btn btn-primary" style="padding: 0 24px; font-weight: bold;" @click="startScan">
              Scan Now
            </button>
          </div>
          <div v-else class="w-full flex-col gap-2">
            <div class="flex justify-between items-center w-full">
              <span class="text-primary font-bold" style="display: flex; gap: 8px; align-items: center;">
                <span class="spinner-small"></span> Scanning in progress...
              </span>
              <span class="text-tertiary font-sm">{{ formatNumber(scanProgress.files) }} files</span>
            </div>
            <div style="width: 100%; background: rgba(0,0,0,0.3); height: 6px; border-radius: 3px; overflow: hidden; position: relative;">
              <div class="scan-bar"></div>
            </div>
            <div class="text-secondary font-sm truncate text-right w-full" :title="scanProgress.path">
              {{ scanProgress.path }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Stats Row -->
    <div class="stats-grid">
      <div class="stat-card" style="--card-color: var(--color-primary);">
        <div class="stat-icon" style="background: rgba(79, 142, 247, 0.15); color: var(--color-primary);">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
          </svg>
        </div>
        <div>
          <div class="stat-label">Total Files</div>
          <div class="stat-value">{{ formatNumber(totalFiles) }}</div>
        </div>
      </div>
      
      <div class="stat-card" style="--card-color: var(--color-accent-purple);">
        <div class="stat-icon" style="background: rgba(155, 109, 255, 0.15); color: var(--color-accent-purple);">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
            <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
          </svg>
        </div>
        <div>
          <div class="stat-label">Total Size</div>
          <div class="stat-value">{{ formatSize(totalSize) }}</div>
        </div>
      </div>

      <div class="stat-card" style="--card-color: var(--color-accent-green);">
        <div class="stat-icon" style="background: rgba(0, 229, 160, 0.15); color: var(--color-accent-green);">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
            <polyline points="22 4 12 14.01 9 11.01"></polyline>
          </svg>
        </div>
        <div>
          <div class="stat-label">Index Health</div>
          <div class="stat-value">100<span class="stat-unit">%</span></div>
        </div>
      </div>
    </div>

    <!-- Charts Row -->
    <div class="two-col flex" style="display: flex; gap: var(--space-4); flex: 1;">
      <div class="card flex-col" style="flex: 1; display: flex;">
        <div class="card-header">
          <div class="card-title">Storage by Category</div>
        </div>
        <div style="flex: 1; min-height: 300px;">
          <v-chart class="chart" :option="categoryChartOption" autoresize />
        </div>
      </div>
      
      <div class="card flex-col" style="flex: 1; display: flex;">
        <div class="card-header">
          <div class="card-title">Recent Large Files</div>
        </div>
        <div style="flex: 1; overflow-y: auto;">
          <table class="data-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="file in largestFiles" :key="file.id">
                <td class="truncate" style="max-width: 200px;" :title="file.name">{{ file.name }}</td>
                <td><span :class="['badge', 'badge-' + file.category]">{{ file.category }}</span></td>
                <td class="font-mono text-tertiary">{{ formatSize(file.size) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { pictureDir } from '@tauri-apps/api/path'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { PieChart } from 'echarts/charts'
import { TitleComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import VChart from 'vue-echarts'

use([CanvasRenderer, PieChart, TitleComponent, TooltipComponent, LegendComponent])

const totalFiles = ref(0)
const totalSize = ref(0)
const largestFiles = ref<any[]>([])
const categoryStats = ref<any[]>([])

// Scan state
const isScanning = ref(false)
const scanPath = ref('')
const scanProgress = ref({
  files: 0,
  path: ''
})

let unlistenStarted: any = null
let unlistenProgress: any = null
let unlistenCompleted: any = null

const categoryChartOption = ref({
  tooltip: { trigger: 'item', backgroundColor: '#1c2333', borderColor: '#3b72d9', textStyle: { color: '#fff' } },
  legend: { top: '5%', left: 'center', textStyle: { color: 'rgba(255,255,255,0.65)' } },
  series: [
    {
      name: 'Storage',
      type: 'pie',
      radius: ['40%', '70%'],
      avoidLabelOverlap: false,
      itemStyle: {
        borderRadius: 10,
        borderColor: '#0f1117',
        borderWidth: 2
      },
      label: { show: false, position: 'center' },
      emphasis: {
        label: { show: true, fontSize: 18, fontWeight: 'bold', color: '#fff' }
      },
      labelLine: { show: false },
      data: []
    }
  ],
  color: ['#ff7eb3', '#7eb3ff', '#b3ff7e', '#ffe07e', '#ff9e7e', '#7effe0', '#c8c8c8', '#8a8a9e']
})

onMounted(async () => {
  await loadData()
  
  // Set default path
  try {
    scanPath.value = await pictureDir()
  } catch (e) {
    scanPath.value = '/Users/hou/Pictures'
  }
  
  // Setup listeners
  unlistenStarted = await listen('scan:started', () => {
    isScanning.value = true
    scanProgress.value.files = 0
    scanProgress.value.path = 'Initializing scan...'
  })

  unlistenProgress = await listen('scan:progress', (event: any) => {
    isScanning.value = true
    const data = event.payload.Progress
    if (data) {
      scanProgress.value.files = data.files_scanned
      scanProgress.value.path = data.current_path
    }
  })

  unlistenCompleted = await listen('scan:completed', () => {
    scanProgress.value.path = 'Scan completed!'
    setTimeout(() => {
      isScanning.value = false
      loadData()
    }, 2000)
  })
})

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress()
  if (unlistenStarted) unlistenStarted()
  if (unlistenCompleted) unlistenCompleted()
})

const startScan = async () => {
  if (isScanning.value || !scanPath.value) return;
  isScanning.value = true;
  try {
    await invoke('cmd_start_scan', { 
      params: { path: scanPath.value, includeHidden: false } 
    })
  } catch (e) {
    console.error('Failed to start scan:', e)
    isScanning.value = false;
  }
}

const loadData = async () => {
  try {
    const stats: any = await invoke('cmd_get_total_stats')
    totalFiles.value = stats.totalFiles
    totalSize.value = stats.totalSize

    const largest: any = await invoke('cmd_get_largest_files', { n: 10 })
    largestFiles.value = largest

    const cats: any = await invoke('cmd_get_category_stats')
    categoryStats.value = cats

    // Update chart
    categoryChartOption.value.series[0].data = cats.map((c: any) => ({
      value: c.totalSize,
      name: c.category.charAt(0).toUpperCase() + c.category.slice(1)
    }))
  } catch (e) {
    console.error('Failed to load dashboard data:', e)
  }
}

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
.dashboard-page {
  height: 100%;
}
.scan-banner {
  padding: 24px;
}
.max-w-lg {
  max-width: 500px;
}
.scan-bar {
  height: 100%;
  background: var(--color-primary);
  width: 30%;
  border-radius: 3px;
  animation: scanning 1.5s infinite ease-in-out;
}
@keyframes scanning {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(400%); }
}
.spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.2);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
