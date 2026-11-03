import { reactive } from 'vue'

export const dialogState = reactive({
  open: false,
  id: 0,
  kind: 'prompt',
  title: '',
  label: '',
  message: '',
  value: '',
  confirmText: '',
  danger: false,
})

let pending = null

function show(opts) {
  closeDialog() // only one at a time: the previous caller gets "cancelled"
  Object.assign(dialogState, { label: '', message: '', value: '' }, opts, { open: true, id: dialogState.id + 1 })
  return new Promise((resolve) => (pending = resolve))
}

// Called by AppDialog. No argument = cancelled.
export function closeDialog(result) {
  const resolve = pending
  pending = null
  dialogState.open = false
  resolve?.(result)
}

export async function promptDialog({ title, label, value = '', confirmText = 'Save' }) {
  const v = await show({ kind: 'prompt', title, label, value, confirmText, danger: false })
  return typeof v === 'string' ? v.trim() || null : null
}

export async function confirmDialog({ title, message, confirmText = 'Delete', danger = true }) {
  return (await show({ kind: 'confirm', title, message, confirmText, danger })) === true
}
