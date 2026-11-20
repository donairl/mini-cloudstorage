<script>
import { kindOf } from './FileIcon.vue'

const MAX_TEXT_BYTES = 1024 * 1024

// How the browser can preview a file: 'image' | 'video' | 'audio' | 'pdf' | 'text', or null.
export function previewKind(f) {
  const k = kindOf(f)
  if (k === 'code') return f.size_bytes <= MAX_TEXT_BYTES ? 'text' : null
  return ['image', 'video', 'audio', 'pdf'].includes(k) ? k : null
}
</script>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const props = defineProps({ file: { type: Object, required: true } })
const emit = defineEmits(['close'])

const kind = previewKind(props.file)
const src = '/api/files/download/' + props.file.id
const text = ref('')
const pdf = ref('')
const ready = ref(false)
const error = ref('')
const unsupported = () => (error.value = 'This file can’t be previewed in your browser.')
const vFocus = { mounted: (el) => el.focus() }
let gone = false

// Text is rendered as plain text and PDFs go through a blob with a forced type, so nothing a user
// uploads (HTML, SVG, …) is ever served inline from our origin.
onMounted(async () => {
  if (kind !== 'text' && kind !== 'pdf') return
  try {
    const res = await fetch(src)
    if (!res.ok) throw new Error(`Could not load the preview (${res.status})`)
    if (kind === 'text') text.value = await res.text()
    else {
      const url = URL.createObjectURL(new Blob([await res.blob()], { type: 'application/pdf' }))
      if (gone) return URL.revokeObjectURL(url)
      pdf.value = url
    }
    ready.value = true
  } catch (e) {
    error.value = e.message
  }
})
onUnmounted(() => {
  gone = true
  if (pdf.value) URL.revokeObjectURL(pdf.value)
})
</script>

<template>
  <div class="lightbox" role="dialog" aria-modal="true" :aria-label="file.name" @click.self="emit('close')" @keydown.esc="emit('close')">
    <figure>
      <p v-if="error" class="msg" role="alert">{{ error }}</p>
      <img v-else-if="kind === 'image'" :src="src" :alt="file.name" @error="unsupported" />
      <video v-else-if="kind === 'video'" :src="src" controls autoplay @error="unsupported" />
      <audio v-else-if="kind === 'audio'" :src="src" controls autoplay @error="unsupported" />
      <iframe v-else-if="kind === 'pdf' && ready" :src="pdf" :title="file.name" />
      <pre v-else-if="kind === 'text' && ready" tabindex="0">{{ text }}</pre>
      <p v-else class="msg">Loading preview…</p>
      <figcaption>
        <span class="name">{{ file.name }}</span>
        <span class="btns">
          <a class="btn" :href="src" download>Download</a>
          <button type="button" @click="emit('close')" v-focus>Close</button>
        </span>
      </figcaption>
    </figure>
  </div>
</template>

<style scoped>
.lightbox {
  position: fixed;
  inset: 0;
  z-index: 10;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgba(0, 0, 0, 0.8);
}
figure { margin: 0; max-width: 100%; min-width: 0; }
img, video { display: block; max-width: 100%; max-height: 80vh; margin: 0 auto; border-radius: 6px; }
audio { display: block; width: min(480px, 90vw); }
iframe { display: block; width: min(1000px, 92vw); height: 80vh; border: 0; border-radius: 6px; background: #fff; }
pre {
  width: min(900px, 92vw);
  max-height: 80vh;
  margin: 0;
  padding: 1rem;
  overflow: auto;
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  font-size: 0.85rem;
  white-space: pre-wrap;
  word-break: break-word;
}
.msg { margin: 0; padding: 2rem; color: #fff; text-align: center; }
figcaption {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-top: 0.75rem;
  color: #fff;
}
.name { min-width: 0; word-break: break-all; }
.btns { display: flex; flex-shrink: 0; gap: 0.5rem; }
.btn {
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  text-decoration: none;
}
</style>
