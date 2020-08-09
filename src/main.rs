//! An example of finding lines in a greyscale image.
//! If running from the root directory of this crate you can test on the
//! wrench image in /examples by running
//! `cargo run --example hough ./examples/wrench.jpg ./tmp`

use image::{open, Rgb, Luma};
use imageproc::edges::canny;
use imageproc::hough::{detect_lines, draw_polar_lines, LineDetectionOptions, PolarLine};
use imageproc::geometric_transformations::{rotate, Interpolation};
use imageproc::map::map_colors;
use std::cmp;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::ops::{Add, Div};
use std::path::Path;

fn main()
{
    if env::args().len() != 3 {
        panic!("Please enter an input file and a target directory")
    }

    let input_path = env::args().nth(1).unwrap();
    let output_dir = env::args().nth(2).unwrap();

    let input_path = Path::new(&input_path);
    let output_dir = Path::new(&output_dir);

    if !output_dir.is_dir() {
        fs::create_dir(output_dir).expect("Failed to create output directory")
    }

    if !input_path.is_file() {
        panic!("Input file does not exist");
    }

    // Load image and convert to grayscale
    let input_image = open(input_path)
        .expect(&format!("Could not load image at {:?}", input_path))
        .to_luma();

    joe_dev(output_dir, input_image);
    // hough_example();
}


fn joe_dev(output_dir: &std::path::Path,
           input_image: image::ImageBuffer<image::Luma<u8>, std::vec::Vec<u8>>)
{

    let box_size = 750;
    let mut sub_image = image::SubImage::new(&input_image, 0, 0, box_size, box_size);
    let max_x = input_image.dimensions().0 - box_size - 1;
    let max_y = input_image.dimensions().1 - box_size - 1;
    // println!("Dimensions: {} x {}", input_image.dimensions().0, input_image.dimensions().1);
    // println!("Max x: {}\nMax y: {}", max_x, max_y);

    // Sliding window
    for i in 0..input_image.dimensions().0/box_size*2 {
        let x = cmp::min(i * box_size/2, max_x);

        for j in 0..input_image.dimensions().1/box_size*2 {
            let y = cmp::min(j * box_size/2, max_y);

            sub_image.change_bounds(x, y, box_size, box_size);
            let img = sub_image.to_image();

            // Detect edges using Canny algorithm
            let edges = canny(&img, 1., 175.0);

            // Detect lines using Hough transform
            let options = LineDetectionOptions {
                vote_threshold: 150,
                suppression_radius: 8,
            };
            let lines: Vec<PolarLine> = detect_lines(&edges, options);
            if lines.len()==0 {
                continue;
            }

            // Calculate median angle of lines
            let mut angles: Vec<u32> = Vec::new();
            for line in &lines {
                &angles.push(line.angle_in_degrees);
            }
            angles.sort();
            let median = vector_median(&angles) as f32;

            // Calculate barcode probability
            let (bar_prob, most_freq) = barcode_probability(angles);
            println!("Barcode Probability: {}\nMost Frequent Angle: {}\n", bar_prob, most_freq);

            if bar_prob > 75 {
                let white = Rgb::<u8>([255, 255, 255]);
                let green = Rgb::<u8>([0, 255, 0]);
                let black = Rgb::<u8>([0, 0, 0]);

                // Convert edge image to colour
                let color_edges = map_colors(&edges, |p| if p[0] > 0 { white } else { black });

                // Draw lines on top of edge image
                let lines_image = draw_polar_lines(&color_edges, &lines, green);
                let lines_path = output_dir.join(format!("img_{}_{}.png", j, i));

                // Save rotated image
                let rotated_img = rotate(&lines_image,
                                         ((lines_image.dimensions().0/2) as f32, (lines_image.dimensions().1/2) as f32),
                                         std::f32::consts::PI - median.to_radians(),
                                         Interpolation::Nearest,
                                         Rgb::<u8>([255, 255, 255]));

                rotated_img.save(&lines_path).unwrap();
            }
        }
    }
}


fn vector_median(vec: &Vec<u32>) -> u32
{
    let len = vec.len();
    let median;
    if len % 2 == 0 {
        median = (vec[len/2] - 1 + vec[len/2]) / 2;
    }
    else {
        median = vec[len/2];
    }

    return median;
}


/**
 * @param ordered vector of angles
 */
fn barcode_probability(angles: Vec<u32>) -> (u8, u32)
{
    let batch_size = 5;
    let mut last_angle = angles[0];
    let mut num_freq = HashMap::new();

    // Counting frequencies
    for mut angle in angles {
        if angle - last_angle <= batch_size {
            angle = last_angle;
        }
        last_angle = angle;
        num_freq.entry(angle)
            .and_modify(|e| { *e += 1 })
            .or_insert(1);
    }

    let mut most_freq = 0;
    let mut highest_freq = 0;
    for entry in num_freq {
        if entry.1 > highest_freq {
            highest_freq = entry.1;
            most_freq = entry.0;
        }
    }

    // Calculate probability
    let high_thresh = 30;
    let low_thresh = 5;
    let prob = highest_freq * 100 / (high_thresh - low_thresh);

    // (Mini-Max prob, most frequent angle)
    return (cmp::min(100, cmp::max(prob as u8, 0)), most_freq);
}

/*
fn scale_image(image: image::ImageBuffer<image::Luma<u8>, std::vec::Vec<u8>>)
    -> image::ImageBuffer<image::Luma<u8>, std::vec::Vec<u8>>
{
    // Scale image down
    let filter_type = image::imageops::FilterType::Lanczos3;
    let scaler = 4;
    let new_width = image.dimensions().0/scaler;
    let new_height = image.dimensions().1/scaler;
    let scaled_image = imageops::resize(&image, new_width, new_height, filter_type);

    return scaled_image;
}
*/

fn hough_example(output_dir: &std::path::Path,
                 input_image: image::ImageBuffer<image::Luma<u8>, std::vec::Vec<u8>>) {

    // Save grayscale image in output directory
    let gray_path = output_dir.join("grey.png");
    input_image.save(&gray_path).unwrap();

    // Detect edges using Canny algorithm
    let edges = canny(&input_image, 1., 200.0);
    let canny_path = output_dir.join("canny.png");
    edges.save(&canny_path).unwrap();

    // Detect lines using Hough transform
    let options = LineDetectionOptions {
        vote_threshold: 300,
        suppression_radius: 8,
    };
    let lines: Vec<PolarLine> = detect_lines(&edges, options);

    let white = Rgb::<u8>([255, 255, 255]);
    let green = Rgb::<u8>([0, 255, 0]);
    let black = Rgb::<u8>([0, 0, 0]);

    // Convert edge image to colour
    let color_edges = map_colors(&edges, |p| if p[0] > 0 { white } else { black });

    // Draw lines on top of edge image
    let lines_image = draw_polar_lines(&color_edges, &lines, green);
    let lines_path = output_dir.join("lines.png");
    lines_image.save(&lines_path).unwrap();
}
