use image::Rgba;
use ndarray::{
    parallel::prelude::*,
    prelude::*,
};

use super::Image;

/// Generate a 1D Gaussian kernel
fn gaussian_kernel_1d(sigma: f32, radius: usize) -> Vec<f32> {
    let mut kernel = Vec::with_capacity(2 * radius + 1);
    let mut sum = 0.0;
    for i in 0..(2 * radius + 1) {
        let x = i as isize - radius as isize;
        let value = (-0.5 * (x as f32 / sigma).powi(2)).exp();
        kernel.push(value);
        sum += value;
    }
    // Normalize kernel
    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

/// Apply 1D convolution along a specific axis
fn convolve_1d(input: &Array2<f32>, kernel: &[f32], axis: Axis) -> Array2<f32> {
    let radius = kernel.len() / 2;
    let mut output = input.clone(); // clone the shape, fill with zeros

    match axis {
        Axis(1) => {
            // Convolve rows (horizontal blur)
            output.axis_iter_mut(Axis(0))
                .into_par_iter()
                .zip(input.axis_iter(Axis(0)))
                .for_each(|(mut out_row, in_row)| {
                    let len = in_row.len();
                    for x in 0..len {
                        let mut acc = 0.0;
                        for k in 0..kernel.len() {
                            let offset = k as isize - radius as isize;
                            let ix = x as isize + offset;
                            if ix >= 0 && ix < len as isize {
                                acc += kernel[k] * in_row[ix as usize];
                            }
                        }
                        out_row[x] = acc;
                    }
                });
        }

        Axis(0) => {
            // Convolve columns (vertical blur)
            output.axis_iter_mut(Axis(1))
                .into_par_iter()
                .zip(input.axis_iter(Axis(1)))
                .for_each(|(mut out_col, in_col)| {
                    let len = in_col.len();
                    for y in 0..len {
                        let mut acc = 0.0;
                        for k in 0..kernel.len() {
                            let offset = k as isize - radius as isize;
                            let iy = y as isize + offset;
                            if iy >= 0 && iy < len as isize {
                                acc += kernel[k] * in_col[iy as usize];
                            }
                        }
                        out_col[y] = acc;
                    }
                });
        }

        _ => unreachable!(),
    }

    output
}

impl Image {
    pub fn blur(&mut self, radius: usize) {
        if radius == 0 {
            return; // No blur needed
        }
        let sigma = radius as f32 / 3.0;
        let kernel = gaussian_kernel_1d(sigma, radius);

        let t = std::time::Instant::now();
        println!("Generated 1D kernel in {:.2?}", t.elapsed());

        let mut channels = [
            Array2::<f32>::zeros((self.height, self.width)),
            Array2::<f32>::zeros((self.height, self.width)),
            Array2::<f32>::zeros((self.height, self.width)),
            Array2::<f32>::zeros((self.height, self.width)),
        ] as [Array2<f32>; 4];

        for ((y, x), pixel) in self.image.indexed_iter() {
            let Rgba([r, g, b, a]) = *pixel;
            channels[0][[y, x]] = r as f32;
            channels[1][[y, x]] = g as f32;
            channels[2][[y, x]] = b as f32;
            channels[3][[y, x]] = a as f32;
        }

        println!("Converted image to channels in {:.2?}", t.elapsed());

        let blurred: [Array2<f32>; 4] = channels
            .into_par_iter()
            .map(|channel| {
                let blurred_channel = convolve_1d(&channel, &kernel, Axis(1));
                convolve_1d(&blurred_channel, &kernel, Axis(0))
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Expected 4 channels");

        println!("Applied 1D convolutions in {:.2?}", t.elapsed());

        for ((y, x), pixel) in self.image.indexed_iter_mut() {
            let r = blurred[0][[y, x]] as u8;
            let g = blurred[1][[y, x]] as u8;
            let b = blurred[2][[y, x]] as u8;
            let a = blurred[3][[y, x]] as u8;
            *pixel = Rgba([r, g, b, a]);
        }

        println!("Converted channels back to image in {:.2?}", t.elapsed());
    }
}