use std::path::PathBuf;

use ab_glyph::PxScale;
use anyhow::Context;
use clap::Parser;
use image::{
    imageops::{overlay, rotate270},
    ImageBuffer, Rgb,
};
use imageproc::{
    drawing::{draw_line_segment_mut, draw_text_mut},
    rect::Rect,
};

static FONT: &[u8] = include_bytes!("../res/SometypeMono-Regular.ttf");

const MARGIN_RIGHT: u32 = 5;
const MARGIN_LEFT: u32 = 5;
const MARGIN_TOP: u32 = 5;
const INTRACELL_MARGIN_LEFT: u32 = 2;
const INTRACELL_MARGIN_TOP: u32 = 2;

#[derive(Parser)]
struct Opts {
    input_csv: PathBuf,
    output_png: PathBuf,

    #[clap(long, short = 'W', default_value = "1280")]
    width: u32,

    #[clap(long, short = 'H', default_value = "720")]
    height: u32,

    /// font file (ttf) to render legend text. Default is embedded font Dharma Type Sometype Mono
    #[clap(long)]
    font: Option<PathBuf>,

    /// Text size
    #[clap(long, short='s', default_value = "12.0")]
    font_scale: f32,

    #[clap(long, short='Y', default_value = "60")]
    first_row_height: u32,

    #[clap(long, short = 'y', default_value = "16")]
    row_height: u32,

    #[clap(long, short='X', default_value = "80")]
    first_column_width: u32,

    #[clap(long, short = 'x', default_value = "60")]
    column_width: u32,
}

fn getcolour(l: f32, s: f32, h: f32) -> Rgb<u8> {
    use palette::{IntoColor, Lch, Srgb};
    let c: Lch = Lch::from_components((l, s, h));
    let c: palette::rgb::Rgb<palette::encoding::Linear<palette::encoding::Srgb>> = c.into_color();
    let c: palette::rgb::Rgb<_, u8> = Srgb::<u8>::from_linear(c);
    let c: Rgb<u8> = Rgb(c.into_components().into());
    c
}

fn main() -> anyhow::Result<()> {
    let Opts {
        input_csv,
        output_png,
        width,
        height,
        font,
        font_scale,
        first_row_height,
        row_height,
        first_column_width,
        column_width,
    } = Opts::parse();

    if width < MARGIN_LEFT + MARGIN_RIGHT {
        anyhow::bail!("Image width too small")
    }
    if height < MARGIN_TOP {
        anyhow::bail!("Image height too small")
    }

    let font_buf;
    let font = if let Some(ref fontpath) = font {
        font_buf = std::fs::read(fontpath)?;
        ab_glyph::FontRef::try_from_slice(&font_buf).context("Invalid font file content")?
    } else {
        ab_glyph::FontRef::try_from_slice(FONT).unwrap()
    };

    let mut data: Vec<csv::StringRecord> = Vec::with_capacity(16);
    let header: csv::StringRecord;
    {
        let mut csvr = csv::Reader::from_reader(std::fs::File::open(input_csv)?);
        header = csvr.headers()?.clone();
        for csvrow in csvr.records() {
            let x = csvrow?;
            data.push(x);
        }
    }

    if data.is_empty() || data[0].is_empty() {
        anyhow::bail!("Empty csv");
    }

    let ncols = data[0].len();
    let nrows = data.len() + 1;

    let mut img = ImageBuffer::<Rgb<u8>, _>::new(width, height);
    img.fill(255);

    let linecolour = getcolour(0.0, 50.0, 0.0);

    let table_width = ncols as u32 * column_width + first_column_width - column_width;
    let table_height = nrows as u32 * row_height + first_row_height - row_height;

    let getx = |j: usize| -> u32 {
        let mut x = MARGIN_LEFT;
        x += column_width * j as u32;
        if j >= 1 {
            x += first_column_width - column_width;
        }
        x
    };
    let gety = |i: usize| -> u32 {
        let mut y = MARGIN_TOP;
        y += row_height * i as u32;
        if i >= 1 {
            y += first_row_height - row_height;
        }
        y
    };

    for i in 0..=nrows {
        let y = gety(i);
        draw_line_segment_mut(
            &mut img,
            (MARGIN_LEFT as f32, y as f32),
            ((MARGIN_LEFT + table_width) as f32, y as f32),
            linecolour,
        );
    }
    for j in 0..=ncols {
        let x = getx(j);
        draw_line_segment_mut(
            &mut img,
            (x as f32, MARGIN_TOP as f32),
            (x as f32, (MARGIN_TOP + table_height) as f32),
            linecolour,
        );
    }

    for i in 0..nrows {
        for j in 0..ncols {
            let mut text: &str = if i == 0 { &header[j] } else { &data[i - 1][j] };

            let mut do_rotate = false;
            let mut hue = 0.0;
            let mut saturation = 0.0;
            let mut bg_lightness = if i == 0 { 93.0 } else { 100.0 };

            loop {
                macro_rules! handle_numeric_prefix {
                    ($var:ident, $t:ident) => {
                        let Some((v, rest)) = $t.split_once(':') else {
                            text = $t;
                            break;
                        };
                        let Ok(v): Result<f32,_> = v.parse() else {
                            text = $t;
                            break;
                        };
                        $var=v;
                        text = rest;
                    }
                }
                if let Some(t) = text.strip_prefix("rot:") {
                    do_rotate = true;
                    text = t;
                } else if let Some(t) = text.strip_prefix("h=") {
                    handle_numeric_prefix!(hue, t);
                } else if let Some(t) = text.strip_prefix("s=") {
                    handle_numeric_prefix!(saturation, t);
                } else if let Some(t) = text.strip_prefix("l=") {
                    handle_numeric_prefix!(bg_lightness, t);
                } else {
                    break;
                }
            }

            let bg_colour = getcolour(bg_lightness, saturation, hue);
            let fg_lightness = if bg_lightness < 65.0 { 100.0 } else { 0.0 };
            let text_colour = getcolour(fg_lightness, 0.0, 0.0);

            if !do_rotate {
                imageproc::drawing::draw_filled_rect_mut(
                    &mut img,
                    Rect::at(getx(j) as i32 + 1, gety(i) as i32 + 1)
                        .of_size(getx(j + 1) - getx(j) - 1, gety(i + 1) - gety(i) - 1),
                    bg_colour,
                );

                let y = gety(i) + INTRACELL_MARGIN_TOP;
                let x = getx(j) + INTRACELL_MARGIN_LEFT;

                draw_text_mut(
                    &mut img,
                    text_colour,
                    x as i32,
                    y as i32,
                    PxScale::from(font_scale),
                    &font,
                    text,
                );
            } else {
                // rotate
                let subimage_width = gety(i + 1) - gety(i) - 1;
                let subimage_height = getx(j + 1) - getx(j) - 1;

                let mut cell = ImageBuffer::<Rgb<u8>, _>::new(subimage_width, subimage_height);
                imageproc::drawing::draw_filled_rect_mut(
                    &mut cell,
                    Rect::at(0, 0).of_size(subimage_width, subimage_height),
                    bg_colour,
                );
                draw_text_mut(
                    &mut cell,
                    text_colour,
                    INTRACELL_MARGIN_TOP as i32,
                    INTRACELL_MARGIN_LEFT as i32,
                    PxScale::from(font_scale),
                    &font,
                    text,
                );

                overlay(
                    &mut img,
                    &rotate270(&cell),
                    getx(j) as i64 + 1,
                    gety(i) as i64 + 1,
                );
            }
        }
    }

    img.save(output_png)?;

    Ok(())
}
