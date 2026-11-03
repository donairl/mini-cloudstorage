<script setup>
import { ref, watch, nextTick } from 'vue'
import { dialogState as s, closeDialog } from '../ui/dialog'

const el = ref(null)
const input = ref(null)
let downTarget = null

watch(
  () => s.open && s.id,
  async (on) => {
    await nextTick()
    const d = el.value
    if (!on) return d.open && d.close()
    if (!d.open) d.showModal()
    input.value?.focus()
    input.value?.select()
  },
)

function submit() {
  closeDialog(s.kind === 'prompt' ? s.value : true)
}

// Native close (Esc) while still open in state = cancel.
function onClose() {
  if (s.open) closeDialog()
}

// Only a press that starts and ends on the backdrop cancels (not a text-selection drag).
function onClick(e) {
  if (e.target === el.value && downTarget === el.value) closeDialog()
}
</script>

<template>
  <dialog
    ref="el"
    class="dlg"
    aria-labelledby="dlg-title"
    :aria-describedby="s.kind === 'confirm' ? 'dlg-msg' : undefined"
    @close="onClose"
    @pointerdown="(e) => (downTarget = e.target)"
    @click="onClick"
  >
    <form @submit.prevent="submit">
      <h2 id="dlg-title">{{ s.title }}</h2>
      <label v-if="s.kind === 'prompt'" class="field">
        <span>{{ s.label }}</span>
        <input ref="input" v-model="s.value" autocomplete="off" spellcheck="false" />
      </label>
      <p v-else id="dlg-msg">{{ s.message }}</p>
      <div class="actions">
        <button type="button" @click="closeDialog()">Cancel</button>
        <button
          type="submit"
          class="primary"
          :class="{ danger: s.danger }"
          :disabled="s.kind === 'prompt' && !s.value.trim()"
        >
          {{ s.confirmText }}
        </button>
      </div>
    </form>
  </dialog>
</template>

<style scoped>
.dlg {
  width: min(420px, calc(100vw - 2rem));
  max-width: none;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 12px 40px rgb(0 0 0 / 0.18);
}
.dlg::backdrop {
  background: rgb(15 20 25 / 0.35);
  backdrop-filter: blur(3px);
}
.dlg[open] { animation: pop 0.16s ease-out; }
.dlg[open]::backdrop { animation: fade 0.16s ease-out; }
@keyframes pop {
  from { opacity: 0; transform: translateY(8px) scale(0.97); }
}
@keyframes fade {
  from { opacity: 0; }
}
@media (prefers-reduced-motion: reduce) {
  .dlg[open], .dlg[open]::backdrop { animation: none; }
}
form { padding: 1.25rem; }
h2 { margin: 0 0 0.75rem; font-size: 1.1rem; }
p { margin: 0; color: var(--muted); line-height: 1.45; overflow-wrap: anywhere; }
.field { display: grid; gap: 0.35rem; font-size: 0.9rem; color: var(--muted); }
.field input { width: 100%; color: var(--text); background: var(--surface); }
.actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1.25rem; }
.primary.danger { background: var(--danger); border-color: var(--danger); }
</style>
