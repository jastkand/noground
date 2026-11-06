//! End-to-end parity check against the reference cutout produced by Python
//! rembg (bria-rmbg) on the same photo. Skips when the model isn't cached.

use std::path::PathBuf;

use crate::{imaging, models};

fn cached_bria() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("/Users/Shared/noground/models/bria-rmbg/bria-rmbg.onnx")];
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".rembg/models/bria-rmbg/bria-rmbg.onnx"));
    }
    candidates.into_iter().find(|p| p.exists())
}

#[test]
fn bria_parity_with_reference() {
    let Some(model_path) = cached_bria() else {
        eprintln!("bria-rmbg not in ~/.rembg cache — skipping parity test");
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("../testdata/player.png");
    let reference = image::open(root.join("../testdata/player-bria-ref.png"))
        .expect("reference cutout")
        .to_rgba8();
    let spec = models::spec("bria-rmbg").unwrap();

    let img = imaging::load_oriented(&input).unwrap();
    let tensor = imaging::preprocess(&img, spec);
    let s = spec.input_size as usize;

    let mut session = ort::session::Session::builder()
        .unwrap()
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
        .unwrap()
        .commit_from_file(&model_path)
        .unwrap();
    let input_name = session.inputs()[0].name().to_string();
    let input_tensor = ort::value::Tensor::from_array(([1usize, 3, s, s], tensor)).unwrap();
    let outputs = session
        .run(ort::inputs![input_name.as_str() => input_tensor])
        .unwrap();
    let (_, data) = outputs[0].try_extract_tensor::<f32>().unwrap();

    let plane_len = s * s;
    let mask = imaging::postprocess(data, plane_len, img.width(), img.height(), spec).unwrap();
    let composed = imaging::compose(&img, &mask);
    let out_path = root.join("../target/parity-out.png");
    imaging::encode_and_write(&composed, &out_path, crate::settings::OutputFormat::Png).unwrap();

    let out = image::open(&out_path).unwrap().to_rgba8();
    assert_eq!(out.dimensions(), reference.dimensions());

    let mut alpha_sum: u64 = 0;
    let mut rgb_sum: u64 = 0;
    let mut over8: u64 = 0;
    let n = out.pixels().count() as u64;
    for (pa, pb) in out.pixels().zip(reference.pixels()) {
        let da = (pa[3] as i32 - pb[3] as i32).abs();
        let dr = ((pa[0] as i32 - pb[0] as i32).abs()
            + (pa[1] as i32 - pb[1] as i32).abs()
            + (pa[2] as i32 - pb[2] as i32).abs())
            / 3;
        alpha_sum += da as u64;
        rgb_sum += dr as u64;
        if da > 8 {
            over8 += 1;
        }
    }
    let mean_alpha = alpha_sum as f64 / n as f64;
    let mean_rgb = rgb_sum as f64 / n as f64;
    let pct_over8 = over8 as f64 / n as f64 * 100.0;
    eprintln!("mean alpha diff {mean_alpha:.3}, mean rgb diff {mean_rgb:.3}, {pct_over8:.3}% pixels alpha >8/255");

    assert!(mean_alpha <= 2.0, "mean alpha diff {mean_alpha:.3} > 2.0");
    assert!(mean_rgb <= 2.0, "mean rgb diff {mean_rgb:.3} > 2.0");
    assert!(pct_over8 <= 1.0, "{pct_over8:.3}% pixels differ by >8/255 alpha (limit 1%)");
}
