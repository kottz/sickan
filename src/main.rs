use clap::Parser;
use glob::glob;
use image::{GenericImageView, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the background image
    #[arg(short, long)]
    background: PathBuf,

    /// Paths or glob patterns for one or more overlay images
    #[arg(short, long, required = true, num_args = 1.., value_delimiter = ' ')]
    overlays: Vec<String>,

    /// Treat white as transparent
    #[arg(short, long)]
    white_transparent: bool,

    /// Output format (text or json)
    #[arg(long = "print-format", value_name = "FORMAT", default_value = "text")]
    print_format: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct MatchResult {
    x: u32,
    y: u32,
    match_score: f64,
    is_perfect: bool,
    is_border_match: bool,
}

#[derive(Serialize, Deserialize)]
struct ImageInfo {
    filename: String,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize)]
struct OverlayResult {
    image_info: ImageInfo,
    matches: Vec<MatchResult>,
}

#[derive(Serialize, Deserialize)]
struct JsonOutput {
    ekman_version: String,
    background: ImageInfo,
    overlays: Vec<OverlayResult>,
    white_transparent: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let print_json = args.print_format == "json";

    let background = image::open(&args.background)?.to_rgba8();
    let bg_dimensions = background.dimensions();

    let overlay_paths = expand_glob_patterns(&args.overlays)?;
    let mut all_results = Vec::new();

    for overlay_path in overlay_paths {
        let results = process_overlay(&background, &overlay_path, args.white_transparent)?;
        if !print_json {
            println!("\nOverlay: {}", overlay_path.display());
            print_report(&results);
        }
        all_results.push((overlay_path, results));
    }

    if print_json {
        let json_output = generate_json_output(
            &args.background,
            &all_results,
            args.white_transparent,
            bg_dimensions,
        );
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    }

    Ok(())
}

fn expand_glob_patterns(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut expanded_paths = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') || pattern.contains('?') {
            for entry in glob(pattern)? {
                expanded_paths.push(entry?);
            }
        } else {
            expanded_paths.push(PathBuf::from(pattern));
        }
    }

    Ok(expanded_paths)
}

fn process_overlay(
    background: &RgbaImage,
    overlay_path: &PathBuf,
    treat_white_as_transparent: bool,
) -> Result<Vec<MatchResult>, Box<dyn std::error::Error>> {
    let overlay = image::open(overlay_path)?.to_rgba8();
    let results = find_best_matches(background, &overlay, treat_white_as_transparent);
    Ok(results)
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

    results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap());

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

fn generate_json_output(
    background: &PathBuf,
    all_results: &[(PathBuf, Vec<MatchResult>)],
    white_transparent: bool,
    bg_dimensions: (u32, u32),
) -> JsonOutput {
    let background_info = ImageInfo {
        filename: background
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        width: bg_dimensions.0,
        height: bg_dimensions.1,
    };

    let overlays = all_results
        .iter()
        .map(|(path, results)| {
            let overlay = image::open(path).unwrap();
            let dimensions = overlay.dimensions();
            OverlayResult {
                image_info: ImageInfo {
                    filename: path.file_name().unwrap().to_str().unwrap().to_string(),
                    width: dimensions.0,
                    height: dimensions.1,
                },
                matches: results.clone(),
            }
        })
        .collect();

    JsonOutput {
        ekman_version: env!("CARGO_PKG_VERSION").to_string(),
        background: background_info,
        overlays,
        white_transparent,
    }
}
