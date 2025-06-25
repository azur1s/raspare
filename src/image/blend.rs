use super::Image;
use image::{Pixel, Rgb, Rgba};

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
        // If top pixel is fully transparent, return bottom pixel
        // TODO: might not work on some other blend modes
        if top[3] == 0 { return bottom; }

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

        let alpha_final = (bottomf32[3] + topf32[3]) - (bottomf32[3] * topf32[3]);
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

impl Image {
    pub fn blend_images(&mut self, above: &Image, mode: BlendMode) {
        for y in 0..self.height.min(above.height) {
            for x in 0..self.width.min(above.width) {
                let top = above.get_pixel_unchecked(x, y);
                let bottom = self.get_pixel_unchecked(x, y);

                let blended_pixel = mode.blend_pixel(top, bottom);

                self.set_pixel_unchecked(x, y, blended_pixel);
            }
        }
    }
}