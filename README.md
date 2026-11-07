# Noground

Local background removal for macOS. Drop an image (or a folder), get a transparent cutout —
everything runs on-device, nothing leaves the Mac.

Upscayl-style dark UI: sidebar (single / batch / settings), drag-and-drop, before/after
compare slider, batch progress with per-file status.

## How it works

- **Tauri v2** app shell (Vue 3 + TypeScript frontend, Rust backend).
- ONNX inference via the [`ort`](https://ort.pyke.io) crate — ONNX Runtime is statically
  linked into the binary at build time; there is no Python and nothing to bundle.
- Models are the plain `.onnx` files from
  [rembg releases](https://github.com/danielgatis/rembg/releases) (tag `v0.0.0`), downloaded
  on demand (checksum-pinned) into a shared cache at
  `/Users/Shared/noground/models/<key>/<key>.onnx` — one download serves every account on
  the Mac. Falls back to per-user app data when `/Users/Shared` isn't writable.
- The pre/post-processing (resize, per-image max normalization, mask postprocess, composite)
  is a faithful Rust port of rembg's session code — verified against Python rembg's bria
  output (`cargo test`): mean alpha diff 0.12/255, 99.8% of pixels within 8/255.
- Models already downloaded by the Python `rembg` tool (`~/.rembg/models/…`, `~/.u2net/…`)
  are probed and reused — no re-download.

| model | size | notes |
|---|---|---|
| bria-rmbg 2.0 | ~1.0 GB | best edges; **CC BY-NC 4.0 — non-commercial only** |
| birefnet-portrait | ~0.95 GB | people/portraits; MIT |
| isnet-general-use | ~170 MB | general use, lighter; Apache-2.0 |
| u2net | ~170 MB | fast, lower quality; Apache-2.0 |

## Model licenses

The `.onnx` weights are downloaded on demand (not bundled with this repo or
the app), so each model keeps its own license — check it before putting a
cutout to work:

- **bria-rmbg 2.0** ([briaai/RMBG-2.0](https://huggingface.co/briaai/RMBG-2.0)) —
  [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/). Non-commercial
  use only; commercial use requires a
  [commercial agreement with BRIA](https://bria.ai/). If your use is commercial,
  pick one of the other three models instead.
- **birefnet-portrait** ([ZhengPeng7/BiRefNet](https://github.com/ZhengPeng7/BiRefNet)) — MIT.
- **isnet-general-use** ([xuebinqin/DIS](https://github.com/xuebinqin/DIS)) — Apache-2.0.
- **u2net** ([xuebinqin/U-2-Net](https://github.com/xuebinqin/U-2-Net)) — Apache-2.0.

## Development

```bash
npm install
npm run tauri dev      # opens the app window
cargo test             # parity test against the reference cutout (needs bria in ~/.rembg)
npm run tauri build    # Noground.app + dmg in target/release/bundle
```

Notes:

- Dropping a folder processes the images directly inside it; subfolders aren't
  recursed into.
- `rust-toolchain.toml` pins the native `stable-aarch64-apple-darwin` toolchain — the
  machine's global rustup default is x86_64 (Rosetta), which ort has no prebuilt
  ONNX Runtime binaries for.
- First build fetches ONNX Runtime from cdn.pyke.io; needs network once.
- `src-tauri/src/bin/pngdiff.rs` compares two PNGs (mean/max alpha diff) — handy for
  cutout comparisons; ImageMagick's compose ops are unreliable on this machine.

## Test data

`testdata/cat.png` is the test photo — "Tabby cat with blue eyes" by AdinaVoicu on
[Pixabay](https://pixabay.com/en/users/AdinaVoicu-485024/), CC0;
`testdata/cat-bria-ref.png` is the Python-rembg bria-rmbg reference cutout used by the
parity test.

## Assets & tooling

- `assets/logo.svg` — minified logo source. The macOS icon is regenerated from it via the
  Big Sur grid (1024 canvas, 824 artwork, 100px margins, 185px radius) rendered in headless
  Chrome, then `npm run tauri icon <png>`.
- `zoomtest/` — standalone Vite harness that mounts the real `CompareSlider` in a plain
  browser (Tauri APIs stubbed) for headless-Chrome layout verification:
  `npx vite --config zoomtest/vite.config.mjs serve`, then screenshot `http://localhost:1430`.

## Credits

- [rembg](https://github.com/danielgatis/rembg) (MIT) — source of the ONNX models
  (release `v0.0.0` assets) and the reference session code this app's
  pre/post-processing pipeline is ported from.
- [briaai/RMBG-2.0](https://huggingface.co/briaai/RMBG-2.0) — bria-rmbg 2.0 weights
  (CC BY-NC 4.0), built on the [BiRefNet](https://github.com/ZhengPeng7/BiRefNet)
  architecture (MIT).
- [U²-Net](https://github.com/xuebinqin/U-2-Net) and
  [DIS/ISNet](https://github.com/xuebinqin/DIS) (both Apache-2.0) — the u2net and
  isnet-general-use weights.
- [ONNX Runtime](https://github.com/microsoft/onnxruntime) (MIT), statically linked via the
  [`ort`](https://github.com/pykeio/ort) crate (MIT OR Apache-2.0).
- [Tauri v2](https://v2.tauri.app) (MIT OR Apache-2.0) and [Vue 3](https://vuejs.org) (MIT)
  — the app shell.
- [Upscayl](https://upscayl.org) (AGPL-3.0) — design inspiration for the UI.
- Test photo: "Tabby cat with blue eyes" by AdinaVoicu on
  [Pixabay](https://pixabay.com/en/users/AdinaVoicu-485024/), CC0.

## License

Noground's code is released under the [MIT License](LICENSE).

Note the split: the MIT license covers this repository's code, while the model
weights it downloads keep their own licenses (see [Model licenses](#model-licenses) —
in particular, bria-rmbg 2.0 is non-commercial).
