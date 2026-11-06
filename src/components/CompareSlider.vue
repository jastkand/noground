<script setup lang="ts">
import { computed, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'

const props = defineProps<{ before: string; after: string }>()

const container = ref<HTMLElement | null>(null)
const split = ref(50)

const beforeSrc = computed(() => convertFileSrc(props.before))
const afterSrc = computed(() => convertFileSrc(props.after))
// reveal the RIGHT of the divider (result side); original shows through on the left
const clipStyle = computed(() => ({ clipPath: `inset(0 0 0 ${split.value}%)` }))

function onPointerDown(e: PointerEvent) {
  container.value?.setPointerCapture(e.pointerId)
  moveTo(e)
}

function onPointerMove(e: PointerEvent) {
  if (e.buttons & 1) {
    moveTo(e)
  }
}

function moveTo(e: PointerEvent) {
  const rect = container.value?.getBoundingClientRect()
  if (!rect) return
  const pct = ((e.clientX - rect.left) / rect.width) * 100
  split.value = Math.min(95, Math.max(5, pct))
}
</script>

<template>
  <div
    ref="container"
    class="compare"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
  >
    <img class="layer" :src="beforeSrc" alt="original" draggable="false" />
    <!-- result side: checkerboard under the cutout so transparency reads as transparency -->
    <div class="checker" :style="clipStyle"></div>
    <img class="layer overlay" :src="afterSrc" alt="background removed" draggable="false" :style="clipStyle" />
    <div class="divider" :style="{ left: `${split}%` }">
      <div class="handle"></div>
    </div>
    <span class="tag">original</span>
    <span class="tag right">noground</span>
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
  cursor: ew-resize;
  touch-action: none;
}

.layer {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
  max-height: calc(100vh - 220px);
  user-select: none;
}

.layer.overlay {
  position: absolute;
  inset: 0;
}

.checker {
  position: absolute;
  inset: 0;
  background:
    conic-gradient(from 90deg at 50% 50%, #343d44 25%, #3f4a52 0 50%, #343d44 0 75%, #3f4a52 0)
    0 0 / 24px 24px;
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
  font-size: 12px;
}

.handle::before {
  content: '◀ ▶';
  letter-spacing: -3px;
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
</style>
