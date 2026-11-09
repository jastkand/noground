#!/usr/bin/env node
// Regression test for CompareSlider zoom behavior. Serves the zoomtest/
// harness (real component, Tauri stubbed), drives it with headless Chrome,
// and asserts the geometry payload the harness publishes via document.title:
//   - both image layers report identical rects (the WKWebView desync invariant:
//     under zoom both layers must stay glued to one transform)
//   - the synthetic pinch actually scaled the stage (matrix scale > 1)
// Run with `npm test`; override the Chrome binary with CHROME_BIN.

import { spawn, spawnSync } from 'node:child_process'
import { closeSync, existsSync, mkdtempSync, openSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createServer } from 'vite'

const harnessConfig = fileURLToPath(new URL('../zoomtest/vite.config.mjs', import.meta.url))
const EPS = 0.5 // px tolerance for "glued together"

function findChrome() {
  const fromEnv = process.env.CHROME_BIN
  if (fromEnv && existsSync(fromEnv)) return fromEnv
  const absolute = [
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
  ]
  const onPath = ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']
  for (const bin of [...absolute, ...onPath]) {
    if (bin.includes('/')) {
      if (existsSync(bin)) return bin
    } else if (spawnSync('which', [bin], { stdio: 'ignore' }).status === 0) {
      return bin
    }
  }
  return null
}

// harness rects are [x, y, width, height]
function fmt(rect) {
  return `${Math.round(rect[2])}x${Math.round(rect[3])} @ ${Math.round(rect[0])},${Math.round(rect[1])}`
}

const chrome = findChrome()
if (!chrome) {
  console.error('zoomtest: FAIL — no Chrome/Chromium binary found (set CHROME_BIN)')
  process.exit(1)
}

const server = await createServer({ configFile: harnessConfig, logLevel: 'error' })
try {
  await server.listen()
  const url = server.resolvedUrls?.local?.[0]
  if (!url) throw new Error('vite server did not report a URL')

  const userDataDir = mkdtempSync(path.join(tmpdir(), 'zoomtest-chrome-'))
  let dump
  try {
    // stdout goes to a file: chrome's spawned helpers (e.g. the Google updater)
    // inherit fds and can outlive chrome, which would stall a piped stdout
    const outFile = path.join(userDataDir, 'dom.html')
    const outFd = openSync(outFile, 'w')
    try {
      // must be an async spawn: the vite server lives in this process, and a
      // synchronous spawn would block the loop and starve chrome's requests
      await new Promise((resolve, reject) => {
        const child = spawn(
          chrome,
          [
            '--headless=new',
            '--disable-gpu',
            '--disable-background-networking',
            '--disable-component-update',
            '--disable-sync',
            '--no-first-run',
            '--no-default-browser-check',
            `--user-data-dir=${userDataDir}`,
            '--window-size=1024,800',
            // virtual time fast-forwards the harness timers and waits out image fetches
            '--virtual-time-budget=6000',
            '--dump-dom',
            url,
          ],
          { stdio: ['ignore', outFd, 'ignore'] },
        )
        const timer = setTimeout(() => child.kill('SIGKILL'), 60_000)
        child.on('error', (err) => {
          clearTimeout(timer)
          reject(err)
        })
        child.on('exit', (status, signal) => {
          clearTimeout(timer)
          if (signal) reject(new Error('chrome did not finish within 60s'))
          else if (status !== 0) reject(new Error(`chrome exited with status ${status}`))
          else resolve()
        })
      })
      dump = readFileSync(outFile, 'utf8')
    } finally {
      closeSync(outFd)
    }
  } finally {
    rmSync(userDataDir, { recursive: true, force: true })
  }

  const m = dump.match(/<title>(\{.*\})<\/title>/s)
  if (!m) {
    console.error(dump.slice(0, 400))
    throw new Error('harness never published its geometry payload (page JS error?)')
  }
  const data = JSON.parse(m[1])

  const problems = []
  const close = (a, b, eps = EPS) => Math.abs(a - b) <= eps

  const [nw, nh, complete] = data.natural
  if (!complete || !nw || !nh) {
    problems.push(`fixtures not loaded (natural=${nw}x${nh}, complete=${complete})`)
  }

  if (![0, 1, 2, 3].every((i) => close(data.before[i], data.after[i]))) {
    problems.push(
      `layers desynced under zoom: before ${fmt(data.before)} vs after ${fmt(data.after)}`,
    )
  }

  const mat = /matrix\(([^)]+)\)/.exec(data.transform)
  if (!mat) {
    problems.push(`no transform on .zoomer (got "${data.transform}")`)
  } else {
    const [a, , , d] = mat[1].split(',').map(Number)
    if (Math.abs(a - d) > 1e-6 || a <= 1) {
      problems.push(`pinch zoom not applied (transform "${data.transform}")`)
    } else if (!close(data.zoomer[2], data.compare[2] * a, 1)) {
      problems.push(
        `stage not scaled: zoomer ${fmt(data.zoomer)} vs ${data.compare[2]}px * ${a.toFixed(2)}`,
      )
    }
  }

  if (problems.length) throw new Error(`\n  ${problems.join('\n  ')}`)

  const scale = mat ? Number(mat[1].split(',')[0]).toFixed(1) : '?'
  console.log(
    `zoomtest: ok — scale ${scale}x, layers glued (${fmt(data.before)}), fixtures ${nw}x${nh} loaded`,
  )
} catch (err) {
  console.error(`zoomtest: FAIL\n  ${err.message}`)
  process.exitCode = 1
} finally {
  await server.close()
}
