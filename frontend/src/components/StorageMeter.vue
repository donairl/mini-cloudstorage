<script setup>
import { computed, onMounted, ref } from 'vue'
import { get } from '../api.js'
import { formatSize } from '../format.js'

const data = ref(null)

async function refresh() {
  try {
    data.value = await get('/api/storage')
  } catch {
    data.value = null
  }
}

onMounted(refresh)
defineExpose({ refresh })

const quota = computed(() => data.value?.quota_bytes ?? null)
const pct = computed(() => (quota.value ? Math.min(100, (data.value.used_bytes / quota.value) * 100) : 0))
const level = computed(() => (pct.value >= 95 ? 'full' : pct.value >= 80 ? 'high' : ''))
const text = computed(() =>
  quota.value
    ? `${formatSize(data.value.used_bytes)} of ${formatSize(quota.value)} used`
    : `${formatSize(data.value.used_bytes)} used`,
)
</script>

<template>
  <div v-if="data" class="storage-meter" :class="level">
    <span class="label">Storage</span>
    <div
      v-if="quota"
      class="bar"
      role="meter"
      aria-label="Storage used"
      aria-valuemin="0"
      :aria-valuemax="quota"
      :aria-valuenow="Math.min(data.used_bytes, quota)"
      :aria-valuetext="text"
    >
      <div class="fill" :style="{ width: pct + '%' }" />
    </div>
    <span class="text">{{ text }}</span>
    <span v-if="level === 'full'" class="hint">Storage almost full</span>
  </div>
</template>

<style scoped>
.storage-meter {
  --fill: var(--accent);
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  width: 200px;
  max-width: 100%;
  min-width: 0;
  font-size: 0.75rem;
  line-height: 1.2;
}
.storage-meter.high { --fill: #bf8700; }
.storage-meter.full { --fill: var(--danger); }
.label { color: var(--muted); font-weight: 600; }
.bar {
  height: 6px;
  border-radius: 999px;
  background: var(--border);
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: inherit;
  background: var(--fill);
  transition: width 0.3s ease;
}
.text { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.hint { color: var(--danger); font-weight: 600; }
@media (prefers-reduced-motion: reduce) {
  .fill { transition: none; }
}
</style>
