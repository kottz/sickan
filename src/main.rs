use image::{Rgba, RgbaImage};
use rayon::prelude::*;

#[derive(Clone)]
struct MatchResult {
    x: u32,
    y: u32,
    match_score: f64,
    is_perfect: bool,
    is_border_match: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let background_path = "output/stockholm/Internal/B AFB-305.bmp";
    let overlay_path = "output/stockholm/Internal/AnusenBackDörr1-307.bmp";
    let treat_white_as_transparent = true;

    let background = image::open(background_path)?.to_rgba8();
    let overlay = image::open(overlay_path)?.to_rgba8();

    let results = find_best_matches(&background, &overlay, treat_white_as_transparent);

    print_report(&results);

    Ok(())
}

fn find_best_matches(
    background: &RgbaImage,
    overlay: &RgbaImage,
    treat_white_as_transparent: bool,
) -> Vec<MatchResult> {
    let (bg_width, bg_height) = background.dimensions();
    let (ov_width, ov_height) = overlay.dimensions();

    let positions: Vec<(u32, u32)> = (0..=bg_width - ov_width)
        .flat_map(|x| (0..=bg_height - ov_height).map(move |y| (x, y)))
        .collect();

    let mut results: Vec<MatchResult> = positions
        .par_iter()
        .map(|&(x, y)| {
            let match_score =
                calculate_match_score(background, overlay, x, y, treat_white_as_transparent);
            let is_perfect = match_score == 1.0;
            let is_border_match =
                check_border_match(background, overlay, x, y, treat_white_as_transparent);

            MatchResult {
                x,
                y,
                match_score,
                is_perfect,
                is_border_match,
            }
        })
        .collect();

    // Sort results by match_score in descending order
    results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());

    // Return all results, or at least the best match if there are no good matches
    if results.is_empty() || results[0].match_score <= 0.5 {
        vec![results[0].clone()]
    } else {
        results
            .into_iter()
            .filter(|r| r.match_score > 0.5)
            .collect()
    }
}

fn calculate_match_score(
    background: &RgbaImage,
    overlay: &RgbaImage,
    x: u32,
    y: u32,
    treat_white_as_transparent: bool,
) -> f64 {
    let (ov_width, ov_height) = overlay.dimensions();
    let mut matching_pixels = 0;
    let mut total_pixels = 0;

    for ov_y in 0..ov_height {
        for ov_x in 0..ov_width {
            let bg_pixel = background.get_pixel(x + ov_x, y + ov_y);
            let ov_pixel = overlay.get_pixel(ov_x, ov_y);

            if treat_white_as_transparent
                && ov_pixel[0] == 255
                && ov_pixel[1] == 255
                && ov_pixel[2] == 255
            {
                continue;
            }

            total_pixels += 1;
            if bg_pixel == ov_pixel {
                matching_pixels += 1;
            }
        }
    }

    matching_pixels as f64 / total_pixels as f64
}

fn check_border_match(
    background: &RgbaImage,
    overlay: &RgbaImage,
    x: u32,
    y: u32,
    treat_white_as_transparent: bool,
) -> bool {
    let (ov_width, ov_height) = overlay.dimensions();

    for ov_x in 0..ov_width {
        let top_bg = *background.get_pixel(x + ov_x, y);
        let top_ov = *overlay.get_pixel(ov_x, 0);
        let bottom_bg = *background.get_pixel(x + ov_x, y + ov_height - 1);
        let bottom_ov = *overlay.get_pixel(ov_x, ov_height - 1);

        if !pixels_match(top_bg, top_ov, treat_white_as_transparent)
            || !pixels_match(bottom_bg, bottom_ov, treat_white_as_transparent)
        {
            return false;
        }
    }

    for ov_y in 0..ov_height {
        let left_bg = *background.get_pixel(x, y + ov_y);
        let left_ov = *overlay.get_pixel(0, ov_y);
        let right_bg = *background.get_pixel(x + ov_width - 1, y + ov_y);
        let right_ov = *overlay.get_pixel(ov_width - 1, ov_y);

        if !pixels_match(left_bg, left_ov, treat_white_as_transparent)
            || !pixels_match(right_bg, right_ov, treat_white_as_transparent)
        {
            return false;
        }
    }

    true
}

fn pixels_match(bg_pixel: Rgba<u8>, ov_pixel: Rgba<u8>, treat_white_as_transparent: bool) -> bool {
    if treat_white_as_transparent && ov_pixel[0] == 255 && ov_pixel[1] == 255 && ov_pixel[2] == 255
    {
        true
    } else {
        bg_pixel == ov_pixel
    }
}

fn print_report(results: &[MatchResult]) {
    println!("Match Report:");
    for (index, result) in results.iter().enumerate() {
        println!(
            "Match {}: Position: ({}, {}), Score: {:.2}, Perfect: {}, Border Match: {}",
            index + 1,
            result.x,
            result.y,
            result.match_score,
            result.is_perfect,
            result.is_border_match
        );
    }
}
