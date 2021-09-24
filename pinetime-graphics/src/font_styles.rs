use crate::display;
use embedded_graphics::{
    geometry::Size,
    image::ImageRaw,
    mono_font::{
        mapping::ASCII, DecorationDimensions, MonoFont, MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::RgbColor,
};

/// 39x74 pixel 46 point size monospace font
pub const JETBRAINS_FONT_46_POINT_BOLD: MonoFont = MonoFont {
    image: ImageRaw::new_binary(
        include_bytes!("../../res/fonts/jetbrains_font_46_bold.raw"),
        624,
    ),
    glyph_mapping: &ASCII,
    character_size: Size::new(39, 74),
    character_spacing: 0,
    baseline: 61,
    underline: DecorationDimensions::new(61 + 2, 1),
    strikethrough: DecorationDimensions::new(74 / 2, 1),
};

// TODO - rename TextResources or something
#[derive(Debug)]
pub struct FontStyles {
    pub watchface_time_style: MonoTextStyle<'static, display::PixelFormat>,
}

unsafe impl Send for FontStyles {}

impl Default for FontStyles {
    fn default() -> Self {
        FontStyles {
            watchface_time_style: MonoTextStyleBuilder::new()
                .font(&JETBRAINS_FONT_46_POINT_BOLD)
                .text_color(display::PixelFormat::WHITE)
                .build(),
        }
    }
}
