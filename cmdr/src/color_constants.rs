use r3bl_ansi_color::Color as AnsiColor;

pub enum DefaultColors {
    LizardGreen,
    SlateGray,
    SilverMetallic,
    FrozenBlue,
    MoonlightBlue,
    NightBlue,
    GuardsRed,
    Orange,
}

impl DefaultColors {
    pub fn as_ansi_color(&self) -> AnsiColor {
        match self {
            DefaultColors::LizardGreen => AnsiColor::Rgb(20, 244, 0),
            DefaultColors::SlateGray => AnsiColor::Rgb(94, 103, 111),
            DefaultColors::SilverMetallic => AnsiColor::Rgb(213, 217, 220),
            DefaultColors::FrozenBlue => AnsiColor::Rgb(171, 204, 242),
            DefaultColors::MoonlightBlue => AnsiColor::Rgb(31, 36, 46),
            DefaultColors::NightBlue => AnsiColor::Rgb(14, 17, 23),
            DefaultColors::GuardsRed => AnsiColor::Rgb(200, 1, 1),
            DefaultColors::Orange => AnsiColor::Rgb(255, 132, 18),
        }
    }
}
