<script>
// White glyphs drawn on the colored document body (viewBox 0 0 24 24).
const TYPES = {
  image: ['#8250df', '<circle cx="9.5" cy="12" r="1.4"/><path d="M7 19l3.2-4 2.3 2.6 1.6-1.9L17 19z"/>'],
  video: ['#d6336c', '<path d="M10 11.5v6.5l5.5-3.25z"/>'],
  audio: ['#0f9d8a', '<path d="M11 17.6v-6.2l5-1.4v6.2" fill="none" stroke="#fff" stroke-width="1.4"/><circle cx="9.8" cy="17.6" r="1.4"/><circle cx="14.8" cy="16.2" r="1.4"/>'],
  pdf: ['#d1242f', '<text x="12" y="17.6" text-anchor="middle" font-size="5.6" font-weight="700" font-family="system-ui, sans-serif">PDF</text>'],
  archive: ['#9a6700', '<path d="M10.5 2H12v1.5h-1.5zM12 3.5h1.5V5H12zM10.5 5H12v1.5h-1.5zM12 6.5h1.5V8H12zM10.5 8H12v1.5h-1.5z"/><rect x="10.25" y="11" width="3.5" height="4.5" rx=".8" fill="none" stroke="#fff" stroke-width="1.3"/>'],
  sheet: ['#1a7f37', '<g fill="none" stroke="#fff" stroke-width="1.3"><rect x="7.5" y="11" width="9" height="7.5" rx=".8"/><path d="M7.5 14.75h9M11 11v7.5"/></g>'],
  doc: ['#2f6fde', '<path d="M8 11.5h8M8 14.5h8M8 17.5h5" stroke="#fff" stroke-width="1.4" stroke-linecap="round"/>'],
  slides: ['#e16f24', '<g fill="none" stroke="#fff" stroke-width="1.3" stroke-linecap="round"><rect x="7.5" y="11" width="9" height="6" rx=".8"/><path d="M12 17v2.2M10 19.2h4"/></g>'],
  code: ['#475569', '<path d="M10 12l-2.5 2.75L10 17.5M14 12l2.5 2.75L14 17.5" fill="none" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
  generic: ['#8c959f', ''],
}

const EXT = {
  image: 'png jpg jpeg gif webp svg bmp ico heic avif tif tiff',
  video: 'mp4 mov mkv webm avi m4v wmv',
  audio: 'mp3 wav ogg oga flac m4a aac opus',
  pdf: 'pdf',
  archive: 'zip rar 7z tar gz tgz bz2 xz',
  sheet: 'xls xlsx csv tsv ods',
  doc: 'doc docx odt rtf',
  slides: 'ppt pptx odp key',
  code: 'txt md json js mjs cjs ts jsx tsx vue html htm css scss xml yml yaml toml ini sh py rb go rs java kt c h cpp hpp cs php sql log',
}

export function kindOf(f) {
  const m = f.mime_type || ''
  const top = m.split('/')[0]
  if (top === 'image' || top === 'video' || top === 'audio') return top
  if (m === 'application/pdf') return 'pdf'
  if (/zip|rar|7z|tar|compressed/.test(m)) return 'archive'
  if (/spreadsheet|excel|csv/.test(m)) return 'sheet'
  if (/presentation|powerpoint/.test(m)) return 'slides'
  if (/wordprocessing|msword|opendocument\.text|rtf/.test(m)) return 'doc'
  if (top === 'text' || /json|javascript|xml/.test(m)) return 'code'
  const ext = (f.name || '').includes('.') ? f.name.split('.').pop().toLowerCase() : ''
  return Object.keys(EXT).find((k) => EXT[k].split(' ').includes(ext)) || 'generic'
}

// Brand color for a file's type; FileGrid uses it to tint previews.
export function fileColor(f) {
  return TYPES[kindOf(f)][0]
}
</script>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  item: Object,
  folder: Boolean,
  size: { type: Number, default: 24 },
})

const type = computed(() => (props.folder || !props.item ? null : TYPES[kindOf(props.item)]))
</script>

<template>
  <svg
    class="file-icon"
    viewBox="0 0 24 24"
    :width="size"
    :height="size"
    :style="{ color: type ? type[0] : 'var(--accent)' }"
    aria-hidden="true"
    focusable="false"
  >
    <template v-if="!type">
      <path fill="currentColor" d="M2.5 6a2 2 0 0 1 2-2h4.4a2 2 0 0 1 1.4.6L12 6.3h7.5a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2h-15a2 2 0 0 1-2-2z" />
      <path fill="#fff" fill-opacity=".2" d="M2.5 9.5h19V18a2 2 0 0 1-2 2h-15a2 2 0 0 1-2-2z" />
    </template>
    <template v-else>
      <path fill="currentColor" d="M6 2h8.5L20 7.5V20a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z" />
      <path fill="#fff" fill-opacity=".45" d="M14.5 2 20 7.5h-4A1.5 1.5 0 0 1 14.5 6z" />
      <g fill="#fff" v-html="type[1]" />
    </template>
  </svg>
</template>

<style scoped>
.file-icon { display: block; flex-shrink: 0; }
</style>
