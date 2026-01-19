#![allow(dead_code)]

pub mod colors {
    use iced::Color;

    // --- Primary (Raycast Brand Red/Pink) ---
    pub const PRIMARY: Color = Color::from_rgb8(255, 90, 99); // #FF5A63
    pub const ON_PRIMARY: Color = Color::from_rgb8(255, 255, 255);
    pub const PRIMARY_CONTAINER: Color = Color::from_rgb8(63, 29, 31); // Dark Red Wash
    pub const ON_PRIMARY_CONTAINER: Color = Color::from_rgb8(255, 217, 219);

    // --- Secondary (Neutral Action / Selection Gray) ---
    pub const SECONDARY: Color = Color::from_rgb8(142, 142, 147); // #8E8E93
    pub const ON_SECONDARY: Color = Color::from_rgb8(22, 22, 24);
    pub const SECONDARY_CONTAINER: Color = Color::from_rgb8(44, 44, 46); // #2C2C2E (Selected Item)
    pub const ON_SECONDARY_CONTAINER: Color = Color::from_rgb8(229, 229, 231);

    // --- Tertiary (Raycast Teal/Green Accent) ---
    pub const TERTIARY: Color = Color::from_rgb8(105, 226, 207); // #69E2CF
    pub const ON_TERTIARY: Color = Color::from_rgb8(0, 55, 48);
    pub const TERTIARY_CONTAINER: Color = Color::from_rgb8(0, 80, 71);
    pub const ON_TERTIARY_CONTAINER: Color = Color::from_rgb8(166, 246, 235);

    // --- Error ---
    pub const ERROR: Color = Color::from_rgb8(255, 69, 58); // System Red
    pub const ON_ERROR: Color = Color::from_rgb8(255, 255, 255);
    pub const ERROR_CONTAINER: Color = Color::from_rgb8(147, 0, 10);
    pub const ON_ERROR_CONTAINER: Color = Color::from_rgb8(255, 218, 214);

    // --- Surface & Containers (The "Transparent" Dark Look) ---
    // Note: Raycast uses a very deep, almost black gray.
    pub const SURFACE: Color = Color::from_rgb8(22, 22, 24); // #161618 (Main Window Bg)
    pub const SURFACE_DIM: Color = Color::from_rgb8(14, 14, 16);
    pub const SURFACE_BRIGHT: Color = Color::from_rgb8(35, 35, 37);

    // Use these for list items and search bars
    pub const SURFACE_CONTAINER_LOWEST: Color = Color::from_rgb8(0, 0, 0);
    pub const SURFACE_CONTAINER_LOW: Color = Color::from_rgb8(28, 28, 30); // #1C1C1E
    pub const SURFACE_CONTAINER: Color = Color::from_rgb8(37, 37, 40); // #252528
    pub const SURFACE_CONTAINER_HIGH: Color = Color::from_rgb8(46, 46, 49); // #2E2E31
    pub const SURFACE_CONTAINER_HIGHEST: Color = Color::from_rgb8(58, 58, 61); // #3A3A3D

    // --- Outline & Variants ---
    pub const OUTLINE: Color = Color::from_rgb8(56, 56, 58); // Subtle Borders
    pub const OUTLINE_VARIANT: Color = Color::from_rgb8(72, 72, 75);
    pub const ON_SURFACE: Color = Color::from_rgb8(242, 242, 247); // #F2F2F7 (Main Text)
    pub const ON_SURFACE_VARIANT: Color = Color::from_rgb8(174, 174, 178); // #AEAEB2 (Subtext)
}

pub mod typo {
    use iced::Font;
    use iced::font::{Family, Weight};
    use iced::widget::Text;
    use iced::widget::text::LineHeight;

    use crate::design_system::typo;

    // --- 1. Font Definitions ---
    // These names ("Roboto", "Roboto Mono") work only if you load the .ttf bytes in main()

    const ROBOTO_REGULAR: Font = Font {
        family: Family::Name("Roboto"),
        weight: Weight::Normal,
        ..Font::DEFAULT
    };

    const ROBOTO_MEDIUM: Font = Font {
        family: Family::Name("Roboto"),
        weight: Weight::Medium,
        ..Font::DEFAULT
    };

    const ROBOTO_MONO: Font = Font {
        family: Family::Name("Roboto Mono"),
        weight: Weight::Normal,
        ..Font::DEFAULT
    };

    // Type Alias for convenience
    pub type Style = (f32, LineHeight, Font);

    // --- 2. Material 3 Standard Scale ---

    // Display (Regular 400)
    pub const DISPLAY_L: Style = (57.0, LineHeight::Relative(1.12), ROBOTO_REGULAR);
    pub const DISPLAY_M: Style = (45.0, LineHeight::Relative(1.15), ROBOTO_REGULAR);
    pub const DISPLAY_S: Style = (36.0, LineHeight::Relative(1.22), ROBOTO_REGULAR);

    // Headline (Regular 400)
    pub const HEADLINE_L: Style = (32.0, LineHeight::Relative(1.25), ROBOTO_REGULAR);
    pub const HEADLINE_M: Style = (28.0, LineHeight::Relative(1.28), ROBOTO_REGULAR);
    pub const HEADLINE_S: Style = (24.0, LineHeight::Relative(1.33), ROBOTO_REGULAR);

    // Title (Medium 500 for M/S, Regular for L)
    pub const TITLE_L: Style = (22.0, LineHeight::Relative(1.27), ROBOTO_REGULAR);
    pub const TITLE_M: Style = (16.0, LineHeight::Relative(1.50), ROBOTO_MEDIUM);
    pub const TITLE_S: Style = (14.0, LineHeight::Relative(1.43), ROBOTO_MEDIUM);

    // Body (Regular 400)
    pub const BODY_L: Style = (16.0, LineHeight::Relative(1.50), ROBOTO_REGULAR);
    pub const BODY_M: Style = (14.0, LineHeight::Relative(1.43), ROBOTO_REGULAR);
    pub const BODY_S: Style = (12.0, LineHeight::Relative(1.33), ROBOTO_REGULAR);

    // Label (Medium 500)
    pub const LABEL_L: Style = (14.0, LineHeight::Relative(1.43), ROBOTO_MEDIUM);
    pub const LABEL_M: Style = (12.0, LineHeight::Relative(1.33), ROBOTO_MEDIUM);
    pub const LABEL_S: Style = (11.0, LineHeight::Relative(1.45), ROBOTO_MEDIUM);

    // --- 3. Custom Additions ---

    // Monospace / Code (Mirrors Body sizes)
    pub const CODE_L: Style = (16.0, LineHeight::Relative(1.50), ROBOTO_MONO);
    pub const CODE_M: Style = (14.0, LineHeight::Relative(1.43), ROBOTO_MONO);
    pub const CODE_S: Style = (12.0, LineHeight::Relative(1.33), ROBOTO_MONO);

    // Desktop Hero (For very large splash text, larger than Display L)
    pub const HERO: Style = (96.0, LineHeight::Relative(1.10), ROBOTO_REGULAR);

    // --- 4. Semantic Aliases ---
    // Use these to describe intent rather than appearance.
    // This allows you to change all buttons at once later if needed.

    pub const BUTTON: Style = LABEL_L;
    pub const TOOLTIP: Style = BODY_S;
    pub const LINK: Style = BODY_M;

    // 1. The trait to apply the style to the Text widget
    pub trait Typography {
        fn typography(self, style: typo::Style) -> Self;
    }

    impl<'a> Typography for Text<'a> {
        fn typography(self, style: typo::Style) -> Self {
            self.size(style.0).line_height(style.1).font(style.2)
        }
    }

    // 2. The trait to modify the Style Tuple itself
    pub trait StyleModifiers {
        fn italic(self) -> Self;
        fn bold(self) -> Self;
    }

    // Implement it for your Style tuple type: (f32, LineHeight, Font)
    impl StyleModifiers for typo::Style {
        fn italic(self) -> Self {
            let (size, height, mut font) = self;
            // Use the fully qualified path to avoid import errors
            font.style = iced::font::Style::Italic;
            (size, height, font)
        }

        fn bold(self) -> Self {
            let (size, height, mut font) = self;
            font.weight = iced::font::Weight::Bold;
            (size, height, font)
        }
    }
}

pub mod icons {
    pub const XS: f32 = 18.0; // Dense lists / Inline
    pub const SM: f32 = 20.0; // Small buttons
    pub const MD: f32 = 24.0; // Standard M3 (Default)
    pub const LG: f32 = 32.0; // Feature callouts
    pub const XL: f32 = 48.0; // Empty state illustrations
}

pub mod spacing {
    // Spacing (8dp Grid)
    pub const SPACE_NONE: f32 = 0.0;
    pub const SPACE_XXS: f32 = 2.0;
    pub const SPACE_XS: f32 = 4.0;
    pub const SPACE_S: f32 = 8.0;
    pub const SPACE_M: f32 = 16.0;
    pub const SPACE_L: f32 = 24.0;
    pub const SPACE_XL: f32 = 32.0;
}
