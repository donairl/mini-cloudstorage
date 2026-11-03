<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'

const props = defineProps({
  label: { type: String, default: 'Home' },
  disabled: Boolean,
})
const emit = defineEmits(['drop'])

const dragging = ref(false)
let depth = 0 // enter/leave fire for every child element; count them to avoid flicker

const hasFiles = (e) => [...(e.dataTransfer?.types || [])].includes('Files')

function onEnter(e) {
  if (!hasFiles(e)) return
  e.preventDefault()
  depth++
  if (!props.disabled) dragging.value = true
}

function onOver(e) {
  if (!hasFiles(e)) return
  e.preventDefault()
  e.dataTransfer.dropEffect = props.disabled ? 'none' : 'copy'
}

function onLeave(e) {
  if (!hasFiles(e)) return
  depth = Math.max(0, depth - 1)
  if (!depth) dragging.value = false
}

function onDrop(e) {
  if (!hasFiles(e)) return
  e.preventDefault()
  depth = 0
  dragging.value = false
  if (props.disabled) return
  const items = [...(e.dataTransfer.items || [])]
  const files = items.length
    ? items
        .filter((i) => i.kind === 'file' && !i.webkitGetAsEntry?.()?.isDirectory)
        .map((i) => i.getAsFile())
        .filter(Boolean)
    : [...e.dataTransfer.files]
  if (files.length) emit('drop', files)
}

const events = { dragenter: onEnter, dragover: onOver, dragleave: onLeave, drop: onDrop }
onMounted(() => Object.entries(events).forEach(([n, fn]) => window.addEventListener(n, fn)))
onBeforeUnmount(() => Object.entries(events).forEach(([n, fn]) => window.removeEventListener(n, fn)))
</script>

<template>
  <div class="dropzone">
    <slot />
    <Transition name="fade">
      <div v-if="dragging" class="overlay" aria-hidden="true">
        <div class="frame">
          <svg width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 18a4.5 4.5 0 0 1-.6-8.96A6 6 0 0 1 18 8.5a4 4 0 0 1-.5 9.5" />
            <path d="M12 12v9" />
            <path d="m8.5 15.5 3.5-3.5 3.5 3.5" />
          </svg>
          <p class="title">Drop files to upload</p>
          <p class="sub">to {{ label }}</p>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  padding: 12px;
  pointer-events: none;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  backdrop-filter: blur(1px);
}
.frame {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.25rem;
  border: 3px dashed var(--accent);
  border-radius: 16px;
  color: var(--accent);
  text-align: center;
}
.title { margin: 0.75rem 0 0; font-size: 1.5rem; font-weight: 600; }
.sub {
  margin: 0;
  max-width: 90%;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fade-enter-active, .fade-leave-active { transition: opacity 0.15s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
@media (prefers-reduced-motion: reduce) {
  .fade-enter-active, .fade-leave-active { transition: none; }
}
</style>
