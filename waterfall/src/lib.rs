// Copyright 2019-2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! This crate is used to render a waterfall style plot of a heatmap

mod palettes;

use clocksource::{DateTime, Nanoseconds, UnixInstant};
use heatmap::*;
pub use palettes::Palette;

use image::*;
use palettes::*;
use rusttype::{point, Font, PositionedGlyph, Scale as TypeScale};

use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Copy, Clone)]
/// Used to configure various strategies for mapping values to colors
pub enum Scale {
    /// Use a linear mapping
    Linear,
    /// Use a logarithmic mapping
    Logarithmic,
}

pub struct WaterfallBuilder {
    output: String,
    labels: HashMap<u64, String>,
    palette: Palette,
    interval: Duration,
    scale: Scale,
    smooth: Option<f32>,
}

impl WaterfallBuilder {
    pub fn new(target: &str) -> Self {
        Self {
            output: target.to_string(),
            labels: HashMap::new(),
            palette: Palette::Classic,
            interval: Duration::from_secs(60),
            scale: Scale::Linear,
            smooth: None,
        }
    }

    /// Adds a label to the horizontal axis at the specified value
    pub fn label(mut self, value: u64, label: &str) -> Self {
        self.labels.insert(value, label.to_string());
        self
    }

    /// Sets the color palette for the waterfall
    pub fn palette(mut self, palette: Palette) -> Self {
        self.palette = palette;
        self
    }

    /// Select a color scale for the waterfall
    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    /// Set a smoothing on the waterfall which is applied before colorization
    pub fn smooth(mut self, sigma: Option<f32>) -> Self {
        self.smooth = sigma;
        self
    }

    // get the scaled weight for a bucket count / width
    fn weight(&self, count: u64, width: u64) -> f64 {
        match self.scale {
            Scale::Linear => count as f64 / width as f64,
            Scale::Logarithmic => (count as f64 / width as f64).log2(),
        }
    }

    // find the bucket with the highest weight
    fn max_weight(&self, heatmap: &heatmap::Heatmap) -> f64 {
        let mut max_weight = 0.0;
        for slice in heatmap {
            for b in slice {
                let weight = self.weight(b.count().into(), b.high() - b.low() + 1);
                if weight > max_weight {
                    max_weight = weight;
                }
            }
        }
        max_weight
    }

    /// Generate the waterfall from the provided heatmap
    pub fn build(self, heatmap: &heatmap::Heatmap) {
        let height = heatmap.active_slices();
        let width = heatmap.buckets();

        let mut buf = RgbImage::new(width.try_into().unwrap(), height.try_into().unwrap());

        let max_weight = self.max_weight(heatmap);

        let colors = match self.palette {
            Palette::Classic => CLASSIC,
            Palette::Ironbow => IRONBOW,
        };

        let mut labels = HashMap::new();
        for (k, v) in &self.labels {
            labels.insert(*k, v);
        }

        let mut label_keys: Vec<u64> = labels.keys().cloned().collect();
        label_keys.sort_unstable();

        let mut l = 0;

        if let Some(sigma) = self.smooth {
            // NOTE: this won't work properly if the palette is > 256 colors

            // build grayscale buffer
            for (y, slice) in heatmap.into_iter().enumerate() {
                for (x, b) in slice.into_iter().enumerate() {
                    let weight = self.weight(b.count().into(), b.high() - b.low() + 1);
                    let scaled_weight = weight / max_weight;
                    let index = (scaled_weight * (colors.len() - 1) as f64).round() as u8;
                    buf.put_pixel(
                        x.try_into().unwrap(),
                        y.try_into().unwrap(),
                        Rgb([index, index, index]),
                    );
                }
            }

            // apply a blur to smooth
            buf = image::imageops::blur(&buf, sigma);

            // colorize the buffer
            for x in 0..buf.width() {
                for y in 0..buf.height() {
                    let index = buf.get_pixel(x, y).0[0];
                    let color = colors[index as usize];
                    buf.put_pixel(x, y, Rgb([color.r, color.g, color.b]));
                }
            }
        } else {
            // set the pixels in the buffer
            for (y, slice) in heatmap.into_iter().enumerate() {
                for (x, b) in slice.into_iter().enumerate() {
                    let weight = self.weight(b.count().into(), b.high() - b.low() + 1);
                    let scaled_weight = weight / max_weight;
                    let index = (scaled_weight * (colors.len() - 1) as f64).round() as usize;
                    let color = colors[index];
                    buf.put_pixel(
                        x.try_into().unwrap(),
                        y.try_into().unwrap(),
                        Rgb([color.r, color.g, color.b]),
                    );
                }
            }
        }

        // add the horizontal labels across the top
        if !label_keys.is_empty() {
            let slice = heatmap.into_iter().next().unwrap();
            for (x, bucket) in slice.into_iter().enumerate() {
                let value = bucket.high();
                if value >= label_keys[l] {
                    if let Some(label) = labels.get(&label_keys[l]) {
                        render_text(label, 25.0, x, 0, &mut buf);
                        for y in 0..height {
                            buf.put_pixel(
                                x.try_into().unwrap(),
                                y.try_into().unwrap(),
                                Rgb([255, 255, 255]),
                            );
                        }
                    }
                    l += 1;
                    if l >= label_keys.len() {
                        break;
                    }
                }
            }
        }

        // add the timestamp labels along the left side
        let now = UnixInstant::<Nanoseconds<u64>>::now();
        let mut display_time = heatmap.start_at();
        let ntick = (1 + now.duration_since(display_time).as_nanos()
            / heatmap.resolution().as_nanos()) as usize;
        if ntick > heatmap.active_slices() {
            // heatmap only has partial history
            // adjust earliest timestamp to display in Waterfall
            display_time += heatmap
                .resolution()
                .mul_f64((ntick - heatmap.active_slices()) as f64);
        }

        for (y, _) in heatmap.into_iter().enumerate() {
            if heatmap.resolution().as_nanos() >= self.interval.as_nanos() {
                let label = format!("{}", DateTime::from(display_time));
                render_text(&label, 25.0, 0, y + 2, &mut buf);
                for x in 0..width {
                    buf.put_pixel(
                        x.try_into().unwrap(),
                        y.try_into().unwrap(),
                        Rgb([255, 255, 255]),
                    );
                }
            }
            display_time += heatmap.resolution();
        }
        buf.save(&self.output).unwrap();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

fn render_text(string: &str, size: f32, x_pos: usize, y_pos: usize, buf: &mut RgbImage) {
    // load font
    let font_data = dejavu::sans_mono::regular();
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    // size and scaling
    let height: f32 = size;
    let scale = TypeScale {
        x: height * 1.0,
        y: height,
    };

    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    let glyphs: Vec<PositionedGlyph> = font.layout(string, scale, offset).collect();

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                let x = (x as i32 + bb.min.x) as usize;
                let y = (y as i32 + bb.min.y) as usize;
                if v > 0.25 {
                    let x = (x + x_pos).try_into().unwrap();
                    let y = (y + y_pos).try_into().unwrap();
                    if x < buf.width() && y < buf.height() {
                        buf.put_pixel(x, y, Rgb([255, 255, 255]));
                    }
                }
            })
        }
    }
}
