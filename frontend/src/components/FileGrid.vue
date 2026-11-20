<script setup>
import FileIcon, { fileColor } from './FileIcon.vue'
import { formatSize, formatDate } from '../format'
import { previewKind } from './FilePreview.vue'

defineProps({
  folders: { type: Array, default: () => [] },
  files: { type: Array, default: () => [] },
})
const emit = defineEmits(['open-folder', 'preview', 'share', 'rename', 'remove'])

const isImage = (f) => f.mime_type.startsWith('image/')
const tint = (f) => ({ background: `color-mix(in srgb, ${fileColor(f)} 9%, var(--surface))` })
const ICON = {
  download: 'M8 2.5v7.5m0 0L5 7m3 3 3-3M3 13.5h10',
  rename: 'M10.8 2.7a1.4 1.4 0 0 1 2 2L5.5 12 2.5 13l1-3z',
  share: 'M8 10V2.5m0 0L5.5 5M8 2.5 10.5 5M4 8H3v5.5h10V8h-1',
  remove: 'M2.5 4.5h11M6.5 4.5V3h3v1.5M4 4.5l.7 9h6.6l.7-9',
}
</script>

<template>
  <div class="file-grid">
    <section v-if="folders.length" aria-label="Folders">
      <h2>Folders</h2>
      <div class="grid">
        <div v-for="d in folders" :key="d.id" class="card folder">
          <button type="button" class="open" :title="d.name" @click="emit('open-folder', d.id)">
            <FileIcon folder />
            <span class="name">{{ d.name }}</span>
          </button>
          <div class="acts">
            <button type="button" title="Share" :aria-label="`Share ${d.name}`" @click="emit('share', 'folders', d)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.share" /></svg>
            </button>
            <button type="button" title="Rename" :aria-label="`Rename ${d.name}`" @click="emit('rename', 'folders', d)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.rename" /></svg>
            </button>
            <button type="button" class="danger" title="Delete" :aria-label="`Delete ${d.name}`" @click="emit('remove', 'folders', d)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.remove" /></svg>
            </button>
          </div>
        </div>
      </div>
    </section>

    <section v-if="files.length" aria-label="Files">
      <h2>Files</h2>
      <div class="grid">
        <div v-for="f in files" :key="f.id" class="card file">
          <button v-if="isImage(f)" type="button" class="thumb" :aria-label="`Preview ${f.name}`" @click="emit('preview', f)">
            <!-- ponytail: thumbnail is the full original, add server-side thumbnails if large photos make the grid slow -->
            <img :src="'/api/files/download/' + f.id" loading="lazy" alt="" />
          </button>
          <button v-else-if="previewKind(f)" type="button" class="thumb" :style="tint(f)" :aria-label="`Preview ${f.name}`" @click="emit('preview', f)">
            <FileIcon :item="f" :size="48" />
          </button>
          <div v-else class="thumb" :style="tint(f)">
            <FileIcon :item="f" :size="48" />
          </div>
          <div class="info">
            <FileIcon :item="f" :size="18" />
            <div class="text">
              <span class="name" :title="f.name">{{ f.name }}</span>
              <span class="meta">{{ formatSize(f.size_bytes) }} · {{ formatDate(f.created_at) }}</span>
            </div>
          </div>
          <div class="acts">
            <a :href="'/api/files/download/' + f.id" download title="Download" :aria-label="`Download ${f.name}`">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.download" /></svg>
            </a>
            <button type="button" title="Share" :aria-label="`Share ${f.name}`" @click="emit('share', 'files', f)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.share" /></svg>
            </button>
            <button type="button" title="Rename" :aria-label="`Rename ${f.name}`" @click="emit('rename', 'files', f)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.rename" /></svg>
            </button>
            <button type="button" class="danger" title="Delete" :aria-label="`Delete ${f.name}`" @click="emit('remove', 'files', f)">
              <svg viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON.remove" /></svg>
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
section + section { margin-top: 1.25rem; }
h2 { margin: 0 0 0.6rem; font-size: 0.85rem; font-weight: 600; color: var(--muted); }
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 0.75rem;
}
.card {
  position: relative;
  min-width: 0;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 1px 2px rgba(31, 35, 40, 0.06);
  transition: box-shadow 0.15s, transform 0.15s;
}
.card:hover { box-shadow: 0 4px 14px rgba(31, 35, 40, 0.12); transform: translateY(-2px); }
.name, .meta { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.open {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  width: 100%;
  padding: 0.75rem 0.8rem;
  border: 0;
  border-radius: 9px;
  background: none;
  text-align: left;
  font-weight: 500;
}
.open:hover { background: none; }
.open .name { min-width: 0; }
.thumb {
  display: grid;
  place-items: center;
  width: 100%;
  aspect-ratio: 4 / 3;
  padding: 0;
  overflow: hidden;
  border: 0;
  border-bottom: 1px solid var(--border);
  border-radius: 9px 9px 0 0;
  background: var(--bg);
}
.thumb:hover { background: var(--bg); }
.thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
.open:focus-visible, .thumb:focus-visible { outline-offset: -2px; }
.info { display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.7rem; }
.text { min-width: 0; }
.name { font-size: 0.9rem; }
.meta { margin-top: 0.15rem; font-size: 0.75rem; color: var(--muted); }
.acts {
  position: absolute;
  top: 0.4rem;
  right: 0.4rem;
  display: flex;
  gap: 0.25rem;
  opacity: 0;
  transition: opacity 0.15s;
}
.folder .acts { top: 50%; transform: translateY(-50%); }
.card:hover .acts, .card:focus-within .acts { opacity: 1; }
.acts button, .acts a {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 1px 2px rgba(31, 35, 40, 0.08);
}
.acts button:hover, .acts a:hover { background: var(--bg); }
.acts .danger { color: var(--danger); }
.acts svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}
@media (hover: none) {
  .acts { opacity: 1; }
  .acts button, .acts a { width: 34px; height: 34px; }
  .folder .open { padding-right: 8rem; }
}
@media (max-width: 400px) {
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.5rem; }
}
@media (prefers-reduced-motion: reduce) {
  .card, .acts { transition: none; }
  .card:hover { transform: none; }
}
</style>
