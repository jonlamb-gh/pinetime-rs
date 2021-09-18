Taken from https://github.com/JF002/InfiniTime/tree/master/src/displayapp/fonts

```bash
sudo apt install otf2bdf

otf2bdf JetBrainsMono-Bold.ttf -p 46 -o JetBrainsMono-Bold-46.bdf
```

Use [bdf-to-mono](https://github.com/embedded-graphics/embedded-graphics/tree/master/tools/bdf-to-mono).

```bash
bdf-to-mono JetBrainsMono-Bold-46.bdf JETBRAINS_FONT_46_POINT_BOLD --png jetbrains_font_46_bold.png

convert jetbrains_font_46_bold.png -depth 1 gray:jetbrains_font_46_bold.raw
```

```rust
/// 39x74 pixel 46 point size monospace font
pub const JETBRAINS_FONT_46_POINT_BOLD: MonoFont = MonoFont {
    image: ImageRaw::new_binary(include_bytes!("../fonts/jetbrains_font_46_bold.raw"), 624),
    glyph_mapping: &ASCII,
    character_size: Size::new(39, 74),
    character_spacing: 0,
    baseline: 61,
    underline: DecorationDimensions::new(61 + 2, 1),
    strikethrough: DecorationDimensions::new(74 / 2, 1),
};
```
