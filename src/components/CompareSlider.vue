<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'

const props = defineProps<{ before: string; after: string }>()

const container = ref<HTMLElement | null>(null)
const split = ref(50)
const scale = ref(1)
const tx = ref(0)
const ty = ref(0)
const containerW = ref(0)
const mode = ref<'none' | 'divider' | 'pan'>('none')
const last = ref({ x: 0, y: 0 })

const beforeSrc = computed(() => convertFileSrc(props.before))
const afterSrc = computed(() => convertFileSrc(props.after))
const stageStyle = computed(() => ({
  transform: `translate(${tx.value}px, ${ty.value}px) scale(${scale.value})`,
}))
const scaleLabel = computed(() => (scale.value === 1 ? 'Fit' : `${Math.round(scale.value * 100)}%`))
const cursor = computed(() =>
  mode.value === 'pan' ? 'grabbing' : scale.value > 1 ? 'grab' : 'ew-resize',
)

// The cutout half is clipped in the zoomer's LOCAL coordinates: map the
// divider's screen x back through the transform. This keeps the clip edge
// glued to the divider in every engine (no ancestor clip over a
// transformed subtree — WebKit misrasterizes that pair).
const cutoutClip = computed(() => {
  const w = containerW.value
  if (!w) return { clipPath: `inset(0 0 0 ${split.value}%)` }
  const screenX = (split.value / 100) * w
  const localX = (screenX - tx.value) / scale.value
  return { clipPath: `inset(0 0 0 ${localX.toFixed(1)}px)` }
})

let observer: ResizeObserver | null = null
onMounted(() => {
  observer = new ResizeObserver(([entry]) => {
    containerW.value = entry.contentRect.width
  })
  if (container.value) observer.observe(container.value)
})
onUnmounted(() => observer?.disconnect())

function clampPan() {
  const rect = container.value?.getBoundingClientRect()
  if (!rect) return
  const minX = rect.width * (1 - scale.value)
  const minY = rect.height * (1 - scale.value)
  tx.value = Math.min(0, Math.max(minX, tx.value))
  ty.value = Math.min(0, Math.max(minY, ty.value))
}

function zoomAt(factor: number, px: number, py: number) {
  const next = Math.min(10, Math.max(1, scale.value * factor))
  if (next === scale.value) return
  const k = next / scale.value
  // keep the point under the cursor fixed while scaling
  tx.value = px - k * (px - tx.value)
  ty.value = py - k * (py - ty.value)
  scale.value = next
  clampPan()
}

function onWheel(e: WheelEvent) {
  // trackpad pinch arrives as wheel events with ctrlKey set
  if (!e.ctrlKey && !e.metaKey) return
  e.preventDefault()
  const rect = container.value?.getBoundingClientRect()
  if (!rect) return
  zoomAt(Math.exp(-e.deltaY * 0.01), e.clientX - rect.left, e.clientY - rect.top)
}

function zoomStep(factor: number) {
  const rect = container.value?.getBoundingClientRect()
  if (!rect) return
  zoomAt(factor, rect.width / 2, rect.height / 2)
}

function resetZoom() {
  scale.value = 1
  tx.value = 0
  ty.value = 0
}

function moveDivider(e: PointerEvent) {
  const rect = container.value?.getBoundingClientRect()
  if (!rect) return
  const pct = ((e.clientX - rect.left) / rect.width) * 100
  split.value = Math.min(95, Math.max(5, pct))
}

function startDivider(e: PointerEvent) {
  mode.value = 'divider'
  container.value?.setPointerCapture(e.pointerId)
  moveDivider(e)
}

function startDrag(e: PointerEvent) {
  if (scale.value > 1) {
    mode.value = 'pan'
    last.value = { x: e.clientX, y: e.clientY }
    container.value?.setPointerCapture(e.pointerId)
  } else {
    startDivider(e)
  }
}

function onPointerMove(e: PointerEvent) {
  if (mode.value === 'divider') {
    moveDivider(e)
  } else if (mode.value === 'pan') {
    tx.value += e.clientX - last.value.x
    ty.value += e.clientY - last.value.y
    last.value = { x: e.clientX, y: e.clientY }
    clampPan()
  }
}

function endDrag() {
  mode.value = 'none'
}
</script>

<template>
  <div
    ref="container"
    class="compare"
    :style="{ cursor }"
    @pointerdown="startDrag"
    @pointermove="onPointerMove"
    @pointerup="endDrag"
    @pointercancel="endDrag"
    @wheel="onWheel"
  >
    <div class="zoomer" :style="stageStyle">
      <img class="layer" :src="beforeSrc" alt="original" draggable="false" />
      <!-- cutout half, clipped in local coords so the edge rides on the divider -->
      <div class="cutout" :style="cutoutClip">
        <div class="checker"></div>
        <img class="layer overlay" :src="afterSrc" alt="background removed" draggable="false" />
      </div>
    </div>
    <div class="divider" :style="{ left: `${split}%` }">
      <div class="handle" title="Drag to compare" @pointerdown.stop="startDivider">
        <svg viewBox="0 0 14 10" width="13" height="9" fill="none" stroke="currentColor"
             stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 1 1 5l3 4" />
          <path d="M10 1l3 4-3 4" />
        </svg>
      </div>
    </div>
    <span class="tag">original</span>
    <span class="tag right">noground</span>
    <div class="zoombar">
      <button class="zb" title="Zoom out" @pointerdown.stop @click="zoomStep(1 / 1.25)">−</button>
      <button class="zb pct" title="Reset zoom" @pointerdown.stop @click="resetZoom">
        {{ scaleLabel }}
      </button>
      <button class="zb" title="Zoom in" @pointerdown.stop @click="zoomStep(1.25)">+</button>
    </div>
  </div>
</template>

<style scoped>
.compare {
  position: relative;
  width: 100%;
  background:
    conic-gradient(from 90deg at 50% 50%, #343d44 25%, #3f4a52 0 50%, #343d44 0 75%, #3f4a52 0)
    0 0 / 24px 24px;
  border-radius: 10px;
  overflow: hidden;
  touch-action: none;
  user-select: none;
}

.zoomer {
  transform-origin: 0 0;
  will-change: transform;
}

.layer {
  display: block;
  width: 100%;
  max-height: calc(100vh - 220px);
  object-fit: contain;
  user-select: none;
  pointer-events: none;
}

.cutout {
  position: absolute;
  inset: 0;
}

.checker,
.cutout .layer.overlay {
  position: absolute;
  inset: 0;
}

.checker {
  background:
    conic-gradient(from 90deg at 50% 50%, #343d44 25%, #3f4a52 0 50%, #343d44 0 75%, #3f4a52 0)
    0 0 / 24px 24px;
}

.cutout .layer.overlay {
  width: 100%;
  height: 100%;
}

.divider {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--text);
  transform: translateX(-1px);
  pointer-events: none;
}

.handle {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: var(--text);
  transform: translate(-50%, -50%);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--bg);
  pointer-events: auto;
  cursor: ew-resize;
}

.tag {
  position: absolute;
  top: 10px;
  left: 10px;
  background: color-mix(in srgb, var(--bg) 75%, transparent);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 2px 8px;
  font-size: 11px;
  color: var(--muted);
  pointer-events: none;
}

.tag.right {
  left: auto;
  right: 10px;
  color: var(--accent);
}

.zoombar {
  position: absolute;
  bottom: 10px;
  right: 10px;
  display: flex;
  gap: 4px;
}

.zb {
  min-width: 28px;
  padding: 3px 8px;
  font-size: 12px;
  background: color-mix(in srgb, var(--bg-deep) 85%, transparent);
  border-color: var(--border);
}

.pct {
  min-width: 52px;
}
</style>
