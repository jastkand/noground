import { createApp, h } from 'vue'
import CompareSlider from '../src/components/CompareSlider.vue'
import './../src/styles/global.css'

createApp({
  render: () => h(CompareSlider, { before: '/before.png', after: '/before.png' }),
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

// report the geometry of both transformed stacks via document.title
setTimeout(() => {
  const [z1, z2] = document.querySelectorAll('.zoomer')
  const r1 = (z1 as HTMLElement).getBoundingClientRect()
  const r2 = (z2 as HTMLElement).getBoundingClientRect()
  const t1 = getComputedStyle(z1).transform
  const t2 = getComputedStyle(z2).transform
  const c = (document.querySelector('.compare') as HTMLElement).getBoundingClientRect()
  const img1 = z1.querySelector('img')!.getBoundingClientRect()
  const img2 = z2.querySelector('img')!.getBoundingClientRect()
  const im1 = z1.querySelector('img') as HTMLImageElement
  const im2 = z2.querySelector('img') as HTMLImageElement
  document.title = JSON.stringify({
    nat1: [im1.naturalWidth, im1.naturalHeight, im1.complete],
    nat2: [im2.naturalWidth, im2.naturalHeight, im2.complete],
    h1: [im1.clientWidth, im1.clientHeight],
    h2: [im2.clientWidth, im2.clientHeight],
    compare: [c.width, c.height],
    z1: [r1.x, r1.y, r1.width, r1.height],
    z2: [r2.x, r2.y, r2.width, r2.height],
    img1: [img1.x, img1.y, img1.width, img1.height],
    img2: [img2.x, img2.y, img2.width, img2.height],
    t1,
    t2,
  })
}, 2500)
