use embedded_graphics::{
    geometry::Size,
    image::ImageRaw,
    mono_font::{mapping::StrGlyphMapping, DecorationDimensions, MonoFont},
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Icon {
    Plug,
    BatteryFull,
    BatteryEmpty,
    BatteryOneQuarter,
    BatteryHalf,
    BatteryThreeQuarter,
}

impl Icon {
    pub fn as_text(self) -> &'static str {
        use Icon::*;
        match self {
            Plug => "\u{F1E6}",
            BatteryFull => "\u{F240}",
            BatteryEmpty => "\u{F244}",
            BatteryOneQuarter => "\u{F243}",
            BatteryHalf => "\u{F242}",
            BatteryThreeQuarter => "\u{F241}",
        }
    }
}

const GLYPH_MAPPING: StrGlyphMapping =
    StrGlyphMapping::new("\u{F001}\u{F015}\u{F017}\u{F024}\u{F027}\u{F028}\u{F029}\u{F03A}\u{F048}\u{F04B}\u{F04C}\u{F04D}\u{F051}\u{F069}\u{F06E}\u{F095}\u{F129}\u{F185}\u{F1E6}\u{F1FC}\u{F201}\u{F21E}\u{F240}\u{F241}\u{F242}\u{F243}\u{F244}\u{F252}\u{F293}\u{F294}\u{F2F2}\u{F3DD}\u{F3FD}\u{F45D}\u{F54B}\u{F54B}\u{F560}\u{F569}\u{F59F}\u{F5A0}\u{F6A9}", '\u{F001}' as _);

/// 27x21 pixel 20 point size monospace icons
pub const FONT_AWESOME_ICONS_20_POINT: MonoFont = MonoFont {
    image: ImageRaw::new_binary(
        include_bytes!("../../res/fonts/font_awesome_icons_20.raw"),
        16 * 27,
    ),
    glyph_mapping: &GLYPH_MAPPING,
    character_size: Size::new(27, 21),
    character_spacing: 0,
    baseline: 21,
    underline: DecorationDimensions::new(22, 1),
    strikethrough: DecorationDimensions::new(18, 1),
};

#[derive(Debug)]
pub struct Icons {
    pub p20: &'static MonoFont<'static>,
}

unsafe impl Send for Icons {}

impl Default for Icons {
    fn default() -> Self {
        Icons {
            p20: &FONT_AWESOME_ICONS_20_POINT,
        }
    }
}
