<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import * as api from '../api'
import { formatSize } from '../format'

const s = ref(null)
const shown = ref(readShown())
watch(shown, (v) => {
  try { localStorage.setItem('overview', v ? 'shown' : 'hidden') } catch {}
})
function readShown() {
  try { return localStorage.getItem('overview') !== 'hidden' } catch { return true }
}

async function refresh() {
  try {
    s.value = await api.get('/api/storage')
  } catch {
    s.value = null
  }
}
onMounted(refresh)
defineExpose({ refresh })

const ICON = {
  storage: 'M3 6c0-1.7 3.1-3 7-3s7 1.3 7 3-3.1 3-7 3-7-1.3-7-3Zm0 0v8c0 1.7 3.1 3 7 3s7-1.3 7-3V6M3 10c0 1.7 3.1 3 7 3s7-1.3 7-3',
  files: 'M5 2.5h6.5L15 6v11.5H5zM11.5 2.5V6H15',
  images: 'M3 4h14v12H3zM3 13l4-4 3 3 2-2 5 5M12.5 7.5h.01',
  videos: 'M3 5h10v10H3zM13 8.5l4-2.5v8l-4-2.5',
  shares: 'M8.5 11.5a3 3 0 0 0 4.2 0l2.6-2.6a3 3 0 0 0-4.2-4.2l-1 1M11.5 8.5a3 3 0 0 0-4.2 0l-2.6 2.6a3 3 0 0 0 4.2 4.2l1-1',
}

const cards = computed(() => {
  const v = s.value
  if (!v) return []
  return [
    {
      key: 'storage',
      label: 'Total uploaded',
      value: formatSize(v.used_bytes),
      sub: v.quota_bytes ? `of ${formatSize(v.quota_bytes)} (${Math.round((v.used_bytes / v.quota_bytes) * 100)}%)` : 'Unlimited storage',
    },
    { key: 'files', label: 'Files', value: v.files, sub: 'across all folders' },
    { key: 'images', label: 'Images', value: v.images, sub: pct(v.images, v.files) },
    { key: 'videos', label: 'Videos', value: v.videos, sub: pct(v.videos, v.files) },
    {
      key: 'shares',
      label: 'Active share links',
      value: v.active_shares,
      sub: v.active_shares
        ? `${v.share_downloads} download${v.share_downloads === 1 ? '' : 's'} · ${v.protected_shares} password-protected`
        : 'Share a file to see it here',
    },
  ]
})

function pct(n, total) {
  return total ? `${Math.round((n / total) * 100)}% of files` : 'No files yet'
}
</script>

<template>
  <section v-if="cards.length" class="stats" aria-labelledby="stats-title">
    <header :class="{ open: shown }">
      <h2 id="stats-title">Overview</h2>
      <button
        type="button"
        class="eye"
        :aria-expanded="shown"
        aria-controls="stats-list"
        :title="shown ? 'Hide overview' : 'Show overview'"
        @click="shown = !shown"
      >
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <path d="M1.5 10S4.5 4 10 4s8.5 6 8.5 6-3 6-8.5 6S1.5 10 1.5 10Z" />
          <circle cx="10" cy="10" r="2.5" />
          <path v-if="!shown" d="M3 3l14 14" />
        </svg>
        <span class="sr-only">{{ shown ? 'Hide overview' : 'Show overview' }}</span>
      </button>
    </header>
    <ul v-show="shown" id="stats-list">
      <li v-for="c in cards" :key="c.key" :class="c.key">
        <span class="icon"><svg viewBox="0 0 20 20" aria-hidden="true"><path :d="ICON[c.key]" /></svg></span>
        <div class="text">
          <span class="label">{{ c.label }}</span>
          <span class="sub">{{ c.sub }}</span>
          <span v-if="c.key === 'storage' && s.quota_bytes" class="bar" aria-hidden="true">
            <span :style="{ width: Math.min(100, (s.used_bytes / s.quota_bytes) * 100) + '%' }" />
          </span>
        </div>
        <span class="value">{{ c.value }}</span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.stats {
  margin-bottom: 1.25rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.45rem 0.6rem 0.45rem 1.1rem;
}
header.open { border-bottom: 1px solid var(--border); }
.eye {
  display: grid;
  place-items: center;
  padding: 0.35rem;
  border: 0;
  border-radius: 8px;
  color: var(--muted);
  background: none;
}
.eye:hover { color: var(--text); background: var(--bg); }
.eye svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
h2 {
  margin: 0;
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted);
}
ul { list-style: none; margin: 0; padding: 0; }
li {
  --tint: var(--accent);
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.8rem 1.1rem;
}
li + li { border-top: 1px solid var(--border); }
li:hover { background: var(--bg); }
.images { --tint: #8250df; }
.videos { --tint: #bf3989; }
.shares { --tint: #1a7f37; }
.files { --tint: #6e7781; }
.icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 10px;
  color: var(--tint);
  background: color-mix(in srgb, var(--tint) 12%, transparent);
}
.icon svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
.text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.15rem; }
.label { font-weight: 600; }
.sub { font-size: 0.82rem; color: var(--muted); }
.value { font-size: 1.25rem; font-weight: 700; font-variant-numeric: tabular-nums; white-space: nowrap; }
.bar {
  display: block;
  max-width: 260px;
  height: 6px;
  margin-top: 0.35rem;
  border-radius: 999px;
  background: var(--bg);
  overflow: hidden;
}
.bar span { display: block; height: 100%; border-radius: inherit; background: var(--tint); }
</style>
