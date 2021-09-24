Taken from https://github.com/JF002/InfiniTime/tree/master/src/displayapp/fonts

```bash
sudo apt install otf2bdf

otf2bdf JetBrainsMono-Bold.ttf -p 46 -o JetBrainsMono-Bold-46.bdf
```

Use [bdf-to-mono](https://github.com/embedded-graphics/embedded-graphics/tree/master/tools/bdf-to-mono).

## `JETBRAINS_FONT_54_POINT_EXTRA_BOLD`

```bash
otf2bdf JetBrainsMono-ExtraBold.ttf -p 54 -w ExtraBold -o JetBrainsMono-ExtraBold-54.bdf
bdf-to-mono JetBrainsMono-ExtraBold-54.bdf JETBRAINS_FONT_54_POINT_EXTRA_BOLD --png jetbrains_font_54_extra_bold.png
convert jetbrains_font_54_extra_bold.png -depth 1 gray:jetbrains_font_54_extra_bold.raw
```

```rust
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
```

## `JETBRAINS_FONT_16_POINT_BOLD`

```bash
otf2bdf JetBrainsMono-Bold.ttf -p 16 -w Bold -o JetBrainsMono-Bold-16.bdf
bdf-to-mono JetBrainsMono-Bold-16.bdf JETBRAINS_FONT_16_POINT_BOLD --png jetbrains_font_16_bold.png
convert jetbrains_font_16_bold.png -depth 1 gray:jetbrains_font_16_bold.raw
```

```rust
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
```
