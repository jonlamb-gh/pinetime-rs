// TODO
//#![deny(warnings)]

use euclid::{Point2D, Rect, Size2D};
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::font::Font;
use font_kit::hinting::HintingOptions;
use image::Luma;
use itertools::Itertools;
use std::sync::Arc;
use std::{fs, path::PathBuf};
use structopt::StructOpt;

/// List of character codes for the icons to be extracted from the font file
const CHARACTER_CODES: [u32; 41] = [
    0xf293, 0xf294, 0xf244, 0xf240, 0xf242, 0xf243, 0xf241, 0xf54b, 0xf21e, 0xf1e6, 0xf54b, 0xf017,
    0xf129, 0xf03a, 0xf185, 0xf560, 0xf001, 0xf3fd, 0xf069, 0xf1fc, 0xf45d, 0xf59f, 0xf5a0, 0xf029,
    0xf027, 0xf028, 0xf6a9, 0xf04b, 0xf04c, 0xf048, 0xf051, 0xf095, 0xf3dd, 0xf04d, 0xf2f2, 0xf024,
    0xf252, 0xf569, 0xf201, 0xf06e, 0xf015,
];

const CHARS_PER_ROW: u32 = 16;

const ABOUT: &str = r#"Generates embedded-graphics MonoFont resources & glyph mappings for icons

Examples:
    # Generate 20 point size icon files font_awesome_icons_20.png and font_awesome_icons_20.raw
    icon-font-gen -i FontAwesome5-Solid+Brands+Regular.woff -o icons/ -s 20 -n font_awesome_icons
"#;

#[derive(Debug, StructOpt)]
#[structopt(about = ABOUT)]
pub struct Opts {
    /// Font file containing the glyphs
    #[structopt(
        name = "font input file",
        long = "input",
        short = "i",
        default_value = "../../res/fonts/FontAwesome5-Solid+Brands+Regular.woff"
    )]
    pub input: PathBuf,

    /// Directory to write the output file(s)
    #[structopt(
        name = "output directory",
        long = "output",
        short = "o",
        default_value = "../../res/fonts"
    )]
    pub output: PathBuf,

    /// Name prefix used for font name and file name.
    /// Point size is appended, e.g. <file_name>_<point_size>
    #[structopt(name = "name", long, short = "n", default_value = "font_awesome_icons")]
    pub name: String,

    /// Font point size
    #[structopt(name = "font point size", long = "size", short = "s")]
    pub size: u8,

    /// Additional character codes to include along with the preset
    #[structopt(name = "character codes", long)]
    pub char_codes: Vec<u32>,
    // TODO - fallback char code
}

#[derive(Debug)]
struct Glyph {
    id: u32,
    raster_rect: Rect<i32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();

    let char_codes: Vec<_> =
        itertools::sorted(CHARACTER_CODES.iter().chain(opts.char_codes.iter())).collect();

    println!("Number of character codes: {}", char_codes.len());

    let font_data = fs::read(&opts.input)?;
    let font = Font::from_bytes(Arc::new(font_data), 0).expect("Error loading font");

    let metrics = font.metrics();
    let offset = (metrics.descent as f32 / metrics.units_per_em as f32 * f32::from(opts.size))
        .round() as i32;
    println!("{:#?}", metrics);
    println!("Offset: {}", offset);

    let glyphs: Vec<_> = char_codes
        .iter()
        .map(|&&c| char::from_u32(c).expect("Bad character code"))
        .map(|chr| {
            font.glyph_for_char(chr).map(|glyph_id| {
                let raster_rect = font
                    .raster_bounds(
                        glyph_id,
                        f32::from(opts.size),
                        &Point2D::zero(),
                        HintingOptions::None,
                        RasterizationOptions::Bilevel,
                    )
                    .expect("Unable to get raster bounds");
                Glyph {
                    id: glyph_id,
                    raster_rect,
                }
            })
        })
        .collect();

    println!("TODO --- {} {:#?}", char_codes[0], glyphs[0]);

    let char_size = Size2D::new(
        glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .as_ref()
                    .map(|glyph| glyph.raster_rect.size.width)
                    .unwrap_or(0)
            })
            .max()
            .unwrap(),
        glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .as_ref()
                    .map(|glyph| glyph.raster_rect.size.height)
                    .unwrap_or(0)
            })
            .max()
            .unwrap(),
    )
    .to_u32();

    println!("Character size: {:?}", char_size);

    let img_size = Size2D::new(
        char_size.width * CHARS_PER_ROW,
        (f64::from(char_size.height) * (glyphs.len() as f64 / f64::from(CHARS_PER_ROW)).ceil())
            as u32,
    );

    println!(
        "Image size: {:?} == {}",
        img_size,
        img_size.width * img_size.height
    );

    let mut imgbuf = image::GrayImage::new(img_size.width, img_size.height);

    for (i, (_chr, glyph)) in char_codes.iter().zip(glyphs.iter()).enumerate() {
        if let Some(glyph) = glyph {
            let mut canvas = Canvas::new(&glyph.raster_rect.size.to_u32(), Format::A8);

            font.rasterize_glyph(
                &mut canvas,
                glyph.id,
                f32::from(opts.size),
                &glyph.raster_rect.origin.to_f32(),
                HintingOptions::None,
                RasterizationOptions::Bilevel,
            )
            .expect("Error rasterizing glyph");

            let col = i as u32 % CHARS_PER_ROW;
            let row = i as u32 / CHARS_PER_ROW;
            let img_x = col * char_size.width;
            let img_y = row * char_size.height + char_size.height;

            // Copy onto image
            for y in (0u32..glyph.raster_rect.size.height as u32)
                .into_iter()
                .rev()
            {
                let (row_start, row_end) =
                    (y as usize * canvas.stride, (y + 1) as usize * canvas.stride);
                let row = &canvas.pixels[row_start..row_end];
                for x in 0u32..glyph.raster_rect.size.width as u32 {
                    let val = row[x as usize];
                    if val != 0 {
                        let pixel_x = img_x as i32 + x as i32 + glyph.raster_rect.origin.x;
                        let pixel_y = img_y as i32 - glyph.raster_rect.size.height + y as i32
                            - glyph.raster_rect.origin.y
                            + offset;
                        if pixel_x >= 0 && pixel_y >= 0 {
                            imgbuf.put_pixel(pixel_x as u32, pixel_y as u32, Luma([0xFFu8]));
                        }
                    }
                }
            }
        }
    }

    let file_name_prefix = format!("{}_{}", opts.name, opts.size);
    let png_file = format!("{}.png", file_name_prefix);
    let png_output_path = opts.output.join(&png_file);

    println!("Writing {}", png_output_path.display());
    imgbuf.save(&png_output_path)?;

    // TODO write raw file
    println!(
        "TODO\nconvert {} -depth 1 gray:{}.raw",
        png_output_path.display(),
        file_name_prefix
    );

    // TODO - do the ranges
    let mappings = char_codes
        .iter()
        .map(|c| format!("\\u{{{:04X}}}", c))
        .collect::<Vec<String>>()
        .join("");
    println!("Mappings: \"{}\"", mappings);

    Ok(())
}

fn consecutive_slices(data: &[u32]) -> Vec<Vec<u32>> {
    (&(0..data.len()).group_by(|&i| data[i] as usize - i))
        .into_iter()
        .map(|(_, group)| group.map(|i| data[i]).collect())
        .collect()
}
