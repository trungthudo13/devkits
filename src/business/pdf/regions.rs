//! Offline region proposals, deliberately separate from semantic image recognition.
use image::{imageops::FilterType, RgbImage};
use std::sync::atomic::{AtomicBool, Ordering};

/// Find large, dense foreground components on a mostly uniform page background.
/// Work is bounded by a 1,000 px thumbnail and a coarse grid, irrespective of PDF DPI.
pub(crate) fn detect(image: &RgbImage, cancel: &AtomicBool) -> Result<Vec<[u32; 4]>, String> {
    let cancelled = || {
        if cancel.load(Ordering::Relaxed) {
            Err("Cancelled.".to_owned())
        } else {
            Ok(())
        }
    };
    cancelled()?;
    let (original_w, original_h) = image.dimensions();
    if original_w < 16 || original_h < 16 {
        return Ok(Vec::new());
    }
    let scale = (1000. / original_w.max(original_h) as f64).min(1.);
    let small = image::imageops::resize(
        image,
        ((original_w as f64 * scale) as u32).max(1),
        ((original_h as f64 * scale) as u32).max(1),
        FilterType::Triangle,
    );
    let (w, h) = small.dimensions();
    // Median border color tolerates edge text, mild scanner noise and tinted paper.
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for (x, y, p) in small.enumerate_pixels() {
        if x < 2 || y < 2 || x + 2 >= w || y + 2 >= h {
            for c in 0..3 {
                channels[c].push(p[c]);
            }
        }
    }
    let background: [u8; 3] = std::array::from_fn(|c| {
        channels[c].sort_unstable();
        channels[c][channels[c].len() / 2]
    });
    const CELL: u32 = 4;
    let gw = w.div_ceil(CELL) as usize;
    let gh = h.div_ceil(CELL) as usize;
    let mut counts = vec![0u8; gw * gh];
    for (x, y, p) in small.enumerate_pixels() {
        if x == 0 {
            cancelled()?;
        }
        if (0..3).any(|c| p[c].abs_diff(background[c]) > 32) {
            counts[y as usize / CELL as usize * gw + x as usize / CELL as usize] += 1;
        }
    }
    let foreground: Vec<_> = counts.iter().map(|&n| n >= 5).collect();
    // Bridge tiny gaps inside photos; leave normal gutters between photos intact.
    let mut occupied = vec![false; gw * gh];
    for y in 0..gh {
        for x in 0..gw {
            if foreground[y * gw + x] {
                for yy in y.saturating_sub(1)..=(y + 1).min(gh - 1) {
                    for xx in x.saturating_sub(1)..=(x + 1).min(gw - 1) {
                        occupied[yy * gw + xx] = true;
                    }
                }
            }
        }
    }
    let mut regions = Vec::new();
    let mut stack = Vec::new();
    for seed in 0..occupied.len() {
        if !occupied[seed] {
            continue;
        }
        cancelled()?;
        stack.push(seed);
        occupied[seed] = false;
        let (mut left, mut top, mut right, mut bottom) = (gw, gh, 0, 0);
        let mut ink = 0usize;
        while let Some(i) = stack.pop() {
            let (x, y) = (i % gw, i / gw);
            if foreground[i] {
                left = left.min(x);
                right = right.max(x + 1);
                top = top.min(y);
                bottom = bottom.max(y + 1);
                ink += counts[i] as usize;
            }
            for (xx, yy) in [
                (x.saturating_sub(1), y),
                ((x + 1).min(gw - 1), y),
                (x, y.saturating_sub(1)),
                (x, (y + 1).min(gh - 1)),
            ] {
                let next = yy * gw + xx;
                if occupied[next] {
                    occupied[next] = false;
                    stack.push(next);
                }
            }
        }
        if left >= right || top >= bottom {
            continue;
        }
        let (bw, bh) = (
            (right - left) * CELL as usize,
            (bottom - top) * CELL as usize,
        );
        let area = bw * bh;
        // Reject text lines, rules, tiny marks and a whole-page background.
        if bw < (w as usize / 12).max(24)
            || bh < (h as usize / 12).max(24)
            || area < (w * h) as usize / 150
            || area > (w * h) as usize * 9 / 10
            || ink * 100 < area * 28
        {
            continue;
        }
        let x = (left as u32 * CELL).saturating_sub(2);
        let y = (top as u32 * CELL).saturating_sub(2);
        let r = (right as u32 * CELL + 2).min(w);
        let b = (bottom as u32 * CELL + 2).min(h);
        let x = x as u64 * original_w as u64 / w as u64;
        let y = y as u64 * original_h as u64 / h as u64;
        let r = (r as u64 * original_w as u64).div_ceil(w as u64);
        let b = (b as u64 * original_h as u64).div_ceil(h as u64);
        regions.push([x as u32, y as u32, (r - x) as u32, (b - y) as u32]);
    }
    // White highlights can split one photo into a main area and narrow side strips.
    // Join only aligned strips, preserving pairs of similarly large photos.
    loop {
        cancelled()?;
        let mut pair = None;
        'search: for i in 0..regions.len() {
            for j in i + 1..regions.len() {
                let (a, b) = (regions[i], regions[j]);
                let overlap = (a[1] + a[3])
                    .min(b[1] + b[3])
                    .saturating_sub(a[1].max(b[1]));
                let gap = a[0]
                    .max(b[0])
                    .saturating_sub((a[0] + a[2]).min(b[0] + b[2]));
                if overlap * 10 >= a[3].max(b[3]) * 8
                    && a[2].min(b[2]) * 6 < original_w
                    && gap <= original_w.min(original_h) / 25
                {
                    pair = Some((i, j));
                    break 'search;
                }
            }
        }
        let Some((i, j)) = pair else {
            break;
        };
        let (a, b) = (regions[i], regions.remove(j));
        let x = a[0].min(b[0]);
        let y = a[1].min(b[1]);
        regions[i] = [
            x,
            y,
            (a[0] + a[2]).max(b[0] + b[2]) - x,
            (a[1] + a[3]).max(b[1] + b[3]) - y,
        ];
    }
    // Remove sparse caption tails that were connected to the photo by a logo or rule.
    for region in &mut regions {
        let left = (region[0] as u64 * w as u64 / original_w as u64) as usize / CELL as usize;
        let right = (((region[0] + region[2]) as u64 * w as u64 / original_w as u64) as usize)
            .div_ceil(CELL as usize)
            .min(gw);
        let top = (region[1] as u64 * h as u64 / original_h as u64) as usize / CELL as usize;
        let bottom = (((region[1] + region[3]) as u64 * h as u64 / original_h as u64) as usize)
            .div_ceil(CELL as usize)
            .min(gh);
        let dense: Vec<_> = (top..bottom)
            .filter(|&y| {
                counts[y * gw + left..y * gw + right]
                    .iter()
                    .map(|&n| n as usize)
                    .sum::<usize>()
                    >= (right - left) * 4
            })
            .collect();
        if let (Some(first), Some(last)) = (dense.first(), dense.last()) {
            let y = ((*first as u32 * CELL).saturating_sub(2) as u64 * original_h as u64 / h as u64)
                as u32;
            let b = ((((last + 1) as u32 * CELL + 2).min(h) as u64 * original_h as u64)
                .div_ceil(h as u64)) as u32;
            region[1] = y;
            region[3] = b - y;
        }
    }
    regions.sort_by_key(|r| (r[1], r[0]));
    regions.truncate(64);
    cancelled()?;
    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(im: &mut RgbImage, r: [u32; 4], color: [u8; 3]) {
        for y in r[1]..r[1] + r[3] {
            for x in r[0]..r[0] + r[2] {
                im.put_pixel(x, y, image::Rgb(color));
            }
        }
    }
    #[test]
    fn separates_photos_ignores_text_and_preserves_full_resolution() {
        let mut im = RgbImage::from_pixel(1600, 2000, image::Rgb([246, 242, 230]));
        rectangle(&mut im, [100, 200, 500, 400], [100, 110, 120]);
        rectangle(&mut im, [900, 200, 500, 400], [160, 50, 20]);
        for y in [660, 710, 760] {
            rectangle(&mut im, [120, y, 450, 10], [0, 0, 0]);
        }
        let regions = detect(&im, &AtomicBool::new(false)).unwrap();
        assert_eq!(regions.len(), 2, "{regions:?}");
        for (r, x) in regions.iter().zip([100, 900]) {
            assert!(r[0] <= x && r[0] + r[2] >= x + 500);
            assert!(r[1] <= 200 && r[1] + r[3] >= 600);
            assert!(r[2] < 540 && r[3] < 440);
        }
    }
    #[test]
    fn rejoins_aligned_narrow_fragments_without_joining_other_rows() {
        let mut im = RgbImage::from_pixel(1000, 800, image::Rgb([255; 3]));
        rectangle(&mut im, [100, 100, 120, 200], [50; 3]);
        rectangle(&mut im, [240, 100, 300, 200], [80; 3]);
        rectangle(&mut im, [100, 450, 440, 200], [90; 3]);
        let regions = detect(&im, &AtomicBool::new(false)).unwrap();
        assert_eq!(regions.len(), 2, "{regions:?}");
        assert!(regions[0][0] <= 100 && regions[0][0] + regions[0][2] >= 540);
        assert!(regions[0][1] + regions[0][3] < 450);
    }
    #[test]
    fn blank_full_bleed_noise_and_cancellation() {
        let mut im = RgbImage::from_pixel(400, 600, image::Rgb([245, 245, 245]));
        for (x, y, p) in im.enumerate_pixels_mut() {
            *p = image::Rgb([240 + ((x + y) % 10) as u8; 3]);
        }
        assert!(detect(&im, &AtomicBool::new(false)).unwrap().is_empty());
        rectangle(&mut im, [4, 4, 392, 592], [60, 60, 60]);
        assert!(detect(&im, &AtomicBool::new(false)).unwrap().is_empty());
        assert!(detect(&im, &AtomicBool::new(true)).is_err());
    }
}
