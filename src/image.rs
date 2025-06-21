use image::{Pixel, Rgb, Rgba};
use ndarray::prelude::*;

#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub image: Array2<Rgba<u8>>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image {{ width: {}, height: {} }}", self.width, self.height)
    }
}

impl Image {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            image: Array2::from_elem((height, width), Rgba([0, 0, 0, 0])),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
        let (width, height) = image.dimensions();
        let image = Array2::from_shape_fn((height as usize, width as usize), |(y, x)| {
            let pixel = image.get_pixel(x as u32, y as u32);
            Rgba([pixel[0], pixel[1], pixel[2], pixel[3]])
        });

        Ok(Self {
            width: width as usize,
            height: height as usize,
            image,
        })
    }

    pub fn to_file(&self, path: &str) -> Result<(), String> {
        let mut img = image::ImageBuffer::new(self.width as u32, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.image.get((y, x)).unwrap();
                img.put_pixel(x as u32, y as u32, Rgba([pixel[0], pixel[1], pixel[2], pixel[3]]));
            }
        }
        img.save(path).map_err(|e| e.to_string())
    }

    #[inline(always)]
    pub fn get_pixel_unchecked(&self, x: u32, y: u32) -> Rgba<u8> {
        self.image
            .get((y as usize, x as usize))
            .map(|&pixel| pixel)
            .unwrap()
    }

    #[inline(always)]
    pub fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: Rgba<u8>) {
        self.image
            .get_mut((y as usize, x as usize))
            .map(|pixel| *pixel = color)
            .unwrap();
    }

    pub fn blend_images(&mut self, above: &Image, mode: BlendMode) {
        for y in 0..self.height.min(above.height) {
            for x in 0..self.width.min(above.width) {
                let top    = above.get_pixel_unchecked(x as u32, y as u32);
                let bottom = self.get_pixel_unchecked(x as u32, y as u32);

                let blended_pixel = mode.blend_pixel(top, bottom);

                self.set_pixel_unchecked(x, y, blended_pixel);
            }
        }
    }

    pub fn resize_nearest_neighbour(&mut self, new_width: usize, new_height: usize) {
        let mut resized = Array2::from_elem((new_height, new_width), Rgba([0, 0, 0, 0]));
        let x_ratio = self.width as f64 / new_width as f64;
        let y_ratio = self.height as f64 / new_height as f64;

        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = (x as f64 * x_ratio).floor() as usize;
                let src_y = (y as f64 * y_ratio).floor() as usize;
                if src_x < self.width && src_y < self.height {
                    resized[(y, x)] = self.get_pixel_unchecked(src_x as u32, src_y as u32);
                }
            }
        }

        self.image = resized;
        self.width = new_width;
        self.height = new_height;
    }

    pub fn resize_bilinear(&mut self, new_width: usize, new_height: usize) {
        let mut resized = Array2::from_elem((new_height, new_width), Rgba([0, 0, 0, 0]));
        let x_ratio = self.width as f64 / new_width as f64;
        let y_ratio = self.height as f64 / new_height as f64;

        for y in 0..new_height {
            for x in 0..new_width {
                let src_x = x as f64 * x_ratio;
                let src_y = y as f64 * y_ratio;

                let x1 = src_x.floor() as usize;
                let y1 = src_y.floor() as usize;
                let x2 = (src_x.ceil() as usize).min(self.width - 1);
                let y2 = (src_y.ceil() as usize).min(self.height - 1);

                let a = self.get_pixel_unchecked(x1 as u32, y1 as u32);
                let b = self.get_pixel_unchecked(x2 as u32, y1 as u32);
                let c = self.get_pixel_unchecked(x1 as u32, y2 as u32);
                let d = self.get_pixel_unchecked(x2 as u32, y2 as u32);

                let x_diff = src_x - src_x.floor();
                let y_diff = src_y - src_y.floor();

                // Bilinear interpolation
                let cr = (a[0] as f64 * (1.0 - x_diff) * (1.0 - y_diff) +
                         b[0] as f64 * x_diff * (1.0 - y_diff) +
                         c[0] as f64 * (1.0 - x_diff) * y_diff +
                         d[0] as f64 * x_diff * y_diff) as u8;
                let cg = (a[1] as f64 * (1.0 - x_diff) * (1.0 - y_diff) +
                         b[1] as f64 * x_diff * (1.0 - y_diff) +
                         c[1] as f64 * (1.0 - x_diff) * y_diff +
                         d[1] as f64 * x_diff * y_diff) as u8;
                let cb = (a[2] as f64 * (1.0 - x_diff) * (1.0 - y_diff) +
                         b[2] as f64 * x_diff * (1.0 - y_diff) +
                         c[2] as f64 * (1.0 - x_diff) * y_diff +
                         d[2] as f64 * x_diff * y_diff) as u8;
                let ca = (a[3] as f64 * (1.0 - x_diff) * (1.0 - y_diff) +
                         b[3] as f64 * x_diff * (1.0 - y_diff) +
                         c[3] as f64 * (1.0 - x_diff) * y_diff +
                         d[3] as f64 * x_diff * y_diff) as u8;
                resized[(y, x)] = Rgba([cr, cg, cb, ca]);
                resized[(y, x)].apply(|c| c.clamp(0, 255));
            }
        }

        self.image = resized;
        self.width = new_width;
        self.height = new_height;
    }
}

// B = top, A = bottom
#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl BlendMode {
    pub fn blend_pixel(&self, top: Rgba<u8>, bottom: Rgba<u8>) -> Rgba<u8> {
        let topf32 = Rgba([
            top[0] as f32 / 255.0,
            top[1] as f32 / 255.0,
            top[2] as f32 / 255.0,
            top[3] as f32 / 255.0,
        ]);
        let bottomf32 = Rgba([
            bottom[0] as f32 / 255.0,
            bottom[1] as f32 / 255.0,
            bottom[2] as f32 / 255.0,
            bottom[3] as f32 / 255.0,
        ]);

        let alpha_final = bottomf32[3] + topf32[3] - bottomf32[3] * topf32[3];
        if alpha_final == 0.0 {
            return Rgba([0, 0, 0, 0]);
        };

        // Premultiply alpha
        let top_pm = Rgb([
            (topf32[0] * topf32[3]),
            (topf32[1] * topf32[3]),
            (topf32[2] * topf32[3]),
        ]);
        let mut bottom_pm = Rgb([
            (bottomf32[0] * bottomf32[3]),
            (bottomf32[1] * bottomf32[3]),
            (bottomf32[2] * bottomf32[3]),
        ]);

        match self {
            BlendMode::Normal => bottom_pm.apply2(&top_pm, |b, t|
                t + b * (1.0 - topf32[3])),
            BlendMode::Multiply => bottom_pm.apply2(&top_pm, |b, t| t * b),
            BlendMode::Screen => bottom_pm.apply2(&top_pm, |b, t|
                1.0 - (1.0 - t) * (1.0 - b)),
            BlendMode::Overlay => {
                bottom_pm.apply2(&top_pm, |b, t| {
                    if t < 0.5 {
                        2.0 * b * t
                    } else {
                        1.0 - 2.0 * (1.0 - b) * (1.0 - t)
                    }
                })
            },
        };

        // Unmultiply
        let mut final_pixel = Rgba([
            (bottom_pm[0] / alpha_final * 255.0) as u8,
            (bottom_pm[1] / alpha_final * 255.0) as u8,
            (bottom_pm[2] / alpha_final * 255.0) as u8,
            (alpha_final * 255.0) as u8,
        ]);
        final_pixel.apply(|c| c.clamp(0, 255));
        final_pixel
    }
}