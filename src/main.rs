use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use ab_glyph::PxScale;
use anyhow::Context;
use clap::Parser;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_text_mut, text_size};

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

    /// font scale to render legend text. Default is 14.
    /// Setting it to 0 prevents rendering legend.
    #[clap(long, default_value = "12.0")]
    font_scale: f32,

    #[clap(long, default_value = "16")]
    first_row_height: u32,

    #[clap(long, short = 'Y', default_value = "16")]
    row_height: u32,

    #[clap(long, default_value = "60")]
    first_column_width: u32,

    #[clap(long, short = 'X', default_value = "60")]
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
    let header : csv::StringRecord;
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
    let nrows = data.len()+1;

    let mut img = ImageBuffer::<Rgb<u8>, _>::new(width, height);
    img.fill(255);

    let linecolour = getcolour(0.0, 50.0, 0.0);

    let table_width = ncols as u32 * column_width + first_column_width - column_width;
    let table_height = nrows as u32 * row_height + first_row_height - row_height;

    for i in 0..=nrows {
        let mut y = MARGIN_TOP;
        y += row_height * i as u32;
        if i >= 1 {
            y += first_row_height - row_height;
        }
        imageproc::drawing::draw_line_segment_mut(
            &mut img,
            (MARGIN_LEFT as f32, y as f32),
            ((MARGIN_LEFT + table_width) as f32, y as f32),
            linecolour,
        );
    }
    for j in 0..=ncols {
        let mut x = MARGIN_LEFT;
        x += column_width * j as u32;
        if j >= 1 {
            x += first_column_width - column_width;
        }
        imageproc::drawing::draw_line_segment_mut(
            &mut img,
            (x as f32, MARGIN_TOP as f32),
            (x as f32, (MARGIN_TOP + table_height) as f32),
            linecolour,
        );
    }

    for i in 0..nrows {
        for j in 0..ncols {
            let mut x = MARGIN_LEFT + INTRACELL_MARGIN_LEFT;
            x += column_width * j as u32;
            if i >= 1 {
                x += first_column_width - column_width;
            }

            let mut y = MARGIN_TOP + INTRACELL_MARGIN_TOP;
            y += row_height * i as u32;
            if i >= 1 {
                y += first_row_height - row_height;
            }

            let c = getcolour(0.0, 50.0, 30.0);

            let text = if i == 0 {
                &header[j]
            } else {
                &data[i-1][j]
            };

            draw_text_mut(
                &mut img,
                c,
                x as i32,
                y as i32,
                PxScale::from(font_scale),
                &font,
                text,
            );
        }
    }


    img.save(output_png)?;

    Ok(())
}
