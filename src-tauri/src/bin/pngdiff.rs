use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: pngdiff <a.png> <b.png>");
        std::process::exit(2);
    }
    let a = image::open(&args[1]).expect("open a").to_rgba8();
    let b = image::open(&args[2]).expect("open b").to_rgba8();
    if a.dimensions() != b.dimensions() {
        eprintln!(
            "dimensions differ: {}x{} vs {}x{}",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        );
        std::process::exit(1);
    }

    let mut alpha_sum: u64 = 0;
    let mut rgb_sum: u64 = 0;
    let mut over8: u64 = 0;
    let mut over24: u64 = 0;
    let mut max_alpha: i32 = 0;
    let n = a.pixels().count() as u64;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        let da = (pa[3] as i32 - pb[3] as i32).abs();
        let dr = ((pa[0] as i32 - pb[0] as i32).abs()
            + (pa[1] as i32 - pb[1] as i32).abs()
            + (pa[2] as i32 - pb[2] as i32).abs())
            / 3;
        alpha_sum += da as u64;
        rgb_sum += dr as u64;
        max_alpha = max_alpha.max(da);
        if da > 8 {
            over8 += 1;
        }
        if da > 24 {
            over24 += 1;
        }
    }

    println!(
        "mean alpha diff: {:.2}\nmean rgb diff:   {:.2}\nmax alpha diff:  {max_alpha}\nalpha >8/255:    {:.3}%\nalpha >24/255:   {:.3}%",
        alpha_sum as f64 / n as f64,
        rgb_sum as f64 / n as f64,
        over8 as f64 / n as f64 * 100.0,
        over24 as f64 / n as f64 * 100.0,
    );
    let _ = PathBuf::new();
}
