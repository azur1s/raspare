use image::{Pixel, Rgba};
use ndarray::prelude::*;

pub mod blend;
pub mod effect;

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
    pub fn get_pixel_unchecked(&self, x: usize, y: usize) -> Rgba<u8> {
        self.image
            .get((y, x))
            .map(|&pixel| pixel)
            .unwrap()
    }

    #[inline(always)]
    pub fn get_pixel_or_default(&self, x: usize, y: usize) -> Rgba<u8> {
        self.image
            .get((y, x))
            .cloned()
            .unwrap_or(Rgba([0, 0, 0, 0]))
    }

    #[inline(always)]
    pub fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: Rgba<u8>) {
        self.image
            .get_mut((y, x))
            .map(|pixel| *pixel = color)
            .unwrap();
    }

    pub fn shift_with_empty(&mut self, dx: f64, dy: f64, fract: bool) {
        let (dx, dy) = if fract {
            ((dx as f64 * self.width  as f64).round() as isize,
             (dy as f64 * self.height as f64).round() as isize)
        } else {
            (dx.round() as isize, dy.round() as isize)
        };

        let new_width = (self.width as isize + dx.abs()) as usize;
        let new_height = (self.height as isize + dy.abs()) as usize;
        let mut new_image = Array2::from_elem(
            (new_height, new_width),
            Rgba([0, 0, 0, 0]),
        );

        self.width = new_width;
        self.height = new_height;

        for y in 0..self.height {
            for x in 0..self.width {
                let new_x = (x as isize + dx) as usize;
                let new_y = (y as isize + dy) as usize;
                if new_x < self.width && new_y < self.height {
                    new_image[(new_y, new_x)] = self.get_pixel_or_default(x, y);
                }
            }
        }
        self.image = new_image;
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
                    resized[(y, x)] = self.get_pixel_unchecked(src_x, src_y);
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

                let a = self.get_pixel_unchecked(x1, y1);
                let b = self.get_pixel_unchecked(x2, y1);
                let c = self.get_pixel_unchecked(x1, y2);
                let d = self.get_pixel_unchecked(x2, y2);

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