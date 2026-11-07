import { createApp, h } from 'vue'
import CompareSlider from '../src/components/CompareSlider.vue'
import './../src/styles/global.css'

createApp({
  render: () => h(CompareSlider, { before: '/cat.png', after: '/cat-out.png' }),
}).mount('#app')

// simulate a trackpad pinch (ctrl+wheel) at the viewport center after mount
setTimeout(() => {
  if (window.NO_ZOOM) return
  const el = document.querySelector('.compare') as HTMLElement
  const r = el.getBoundingClientRect()
  for (let i = 0; i < 6; i++) {
    el.dispatchEvent(
      new WheelEvent('wheel', {
        ctrlKey: true,
        deltaY: -120,
        clientX: r.left + r.width / 2,
        clientY: r.top + r.height / 2,
        bubbles: true,
        cancelable: true,
      }),
    )
  }
}, 500)

// report the geometry of the transformed stack and both image layers via
// document.title — under zoom both layers must stay glued to one transform
setTimeout(() => {
  const zoomer = document.querySelector('.zoomer') as HTMLElement
  const [before, after] = [...zoomer.querySelectorAll('img')] as HTMLImageElement[]
  const rect = (el: HTMLElement) => {
    const r = el.getBoundingClientRect()
    return [r.x, r.y, r.width, r.height]
  }
  document.title = JSON.stringify({
    natural: [before.naturalWidth, before.naturalHeight, before.complete],
    compare: rect(document.querySelector('.compare') as HTMLElement),
    zoomer: rect(zoomer),
    before: rect(before),
    after: rect(after),
    transform: getComputedStyle(zoomer).transform,
  })
}, 2500)
