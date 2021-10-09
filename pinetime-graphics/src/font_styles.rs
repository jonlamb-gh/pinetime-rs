use pinetime_common::display;
use pinetime_common::embedded_graphics::{
    geometry::Size,
    image::ImageRaw,
    mono_font::{
        mapping::ASCII, DecorationDimensions, MonoFont, MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::RgbColor,
};

/// 44x85 pixel 54 point size extra bold monospace font
pub const JETBRAINS_FONT_54_POINT_EXTRA_BOLD: MonoFont = MonoFont {
    image: ImageRaw::new_binary(
        include_bytes!("../../res/fonts/jetbrains_font_54_extra_bold.raw"),
        704,
    ),
    glyph_mapping: &ASCII,
    character_size: Size::new(44, 85),
    character_spacing: 2,
    baseline: 71,
    underline: DecorationDimensions::new(71 + 2, 1),
    strikethrough: DecorationDimensions::new(85 / 2, 1),
};

/// 13x25 pixel 16 point size bold monospace font
pub const JETBRAINS_FONT_16_POINT_BOLD: MonoFont = MonoFont {
    image: ImageRaw::new_binary(
        include_bytes!("../../res/fonts/jetbrains_font_16_bold.raw"),
        208,
    ),
    glyph_mapping: &ASCII,
    character_size: Size::new(13, 25),
    character_spacing: 0,
    baseline: 20,
    underline: DecorationDimensions::new(20 + 2, 1),
    strikethrough: DecorationDimensions::new(25 / 2, 1),
};

#[derive(Debug)]
pub struct FontStyles {
    pub watchface_time: Font,
    pub watchface_date: Font,
}

// dyn GlyphMapping + 'static)` cannot be shared between threads safely
unsafe impl Sync for FontStyles {}
unsafe impl Send for FontStyles {}

impl FontStyles {
    pub const fn new() -> Self {
        FontStyles {
            watchface_time: Font {
                font: &JETBRAINS_FONT_54_POINT_EXTRA_BOLD,
                text_color: display::PixelFormat::WHITE,
            },
            watchface_date: Font {
                font: &JETBRAINS_FONT_16_POINT_BOLD,
                //text_color: display::PixelFormat::WHITE,
                text_color: display::PixelFormat::new(
                    display::PixelFormat::MAX_R / 2,
                    display::PixelFormat::MAX_G / 2,
                    display::PixelFormat::MAX_B / 2,
                ),
            },
        }
    }
}

impl Default for FontStyles {
    fn default() -> Self {
        FontStyles::new()
    }
}

#[derive(Debug)]
pub struct Font {
    pub font: &'static MonoFont<'static>,
    pub text_color: display::PixelFormat,
}

// dyn GlyphMapping + 'static)` cannot be shared between threads safely
unsafe impl Sync for Font {}
unsafe impl Send for Font {}

impl Font {
    pub fn style(&self) -> MonoTextStyle<'static, display::PixelFormat> {
        MonoTextStyleBuilder::new()
            .font(self.font)
            .text_color(self.text_color)
            .build()
    }
}
