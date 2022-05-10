use std::{num::ParseFloatError, str::FromStr};

use crate::RenderingMode;
use crossterm::style::{Attribute, Attributes, Color};
use packed_simd::u16x4;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RinkColor {
    pub rgb: packed_simd::u16x4,
    pub alpha: u16,
}

fn parse_value(
    v: &str,
    current_max_output: f32,
    required_max_output: f32,
) -> Result<f32, ParseFloatError> {
    if let Some(stripped) = v.strip_suffix('%') {
        Ok((stripped.trim().parse::<f32>()? / 100.0) * required_max_output)
    } else {
        Ok((v.trim().parse::<f32>()? / current_max_output) * required_max_output)
    }
}

pub struct ParseColorError;

fn parse_hex(color: &str) -> Result<[u8; 3], ParseColorError> {
    let mut values = [0, 0, 0];
    let mut color_ok = true;
    for i in 0..values.len() {
        if let Ok(v) = u8::from_str_radix(&color[(1 + 2 * i)..(1 + 2 * (i + 1))], 16) {
            values[i] = v;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        Ok(values)
    } else {
        Err(ParseColorError)
    }
}

fn parse_rgb(color: &str) -> Result<[u8; 3], ParseColorError> {
    let mut values = [0, 0, 0];
    let mut color_ok = true;
    for (v, i) in color.split(',').zip(0..values.len()) {
        if let Ok(v) = parse_value(v.trim(), 255.0, 255.0) {
            values[i] = v as u8;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        Ok(values)
    } else {
        Err(ParseColorError)
    }
}

fn parse_hsl(color: &str) -> Result<[u8; 3], ParseColorError> {
    let mut values = [0.0, 0.0, 0.0];
    let mut color_ok = true;
    for (v, i) in color.split(',').zip(0..values.len()) {
        if let Ok(v) = parse_value(v.trim(), if i == 0 { 360.0 } else { 100.0 }, 1.0) {
            values[i] = v;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        let [h, s, l] = values;
        let rgb = if s == 0.0 {
            [l as u8; 3]
        } else {
            fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
                if t < 0.0 {
                    t += 1.0;
                }
                if t > 1.0 {
                    t -= 1.0;
                }
                if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 1.0 / 2.0 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                }
            }

            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            [
                (hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0) as u8,
                (hue_to_rgb(p, q, h) * 255.0) as u8,
                (hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0) as u8,
            ]
        };

        Ok(rgb)
    } else {
        Err(ParseColorError)
    }
}

impl FromStr for RinkColor {
    type Err = ParseColorError;

    fn from_str(color: &str) -> Result<Self, Self::Err> {
        match color {
            "red" => Ok(RinkColor {
                rgb: rgb(255, 0, 0),
                alpha: 255,
            }),
            "black" => Ok(RinkColor {
                rgb: rgb(0, 0, 0),
                alpha: 255,
            }),
            "green" => Ok(RinkColor {
                rgb: rgb(0, 255, 0),
                alpha: 255,
            }),
            "yellow" => Ok(RinkColor {
                rgb: rgb(255, 255, 0),
                alpha: 255,
            }),
            "blue" => Ok(RinkColor {
                rgb: rgb(0, 0, 255),
                alpha: 255,
            }),
            "magenta" => Ok(RinkColor {
                rgb: rgb(255, 0, 255),
                alpha: 255,
            }),
            "cyan" => Ok(RinkColor {
                rgb: rgb(0, 255, 255),
                alpha: 255,
            }),
            "gray" => Ok(RinkColor {
                rgb: rgb(128, 128, 128),
                alpha: 255,
            }),
            "darkgray" => Ok(RinkColor {
                rgb: rgb(169, 169, 169),
                alpha: 255,
            }),
            // light red does not exist
            "orangered" => Ok(RinkColor {
                rgb: rgb(255, 69, 0),
                alpha: 255,
            }),
            "lightgreen" => Ok(RinkColor {
                rgb: rgb(144, 238, 144),
                alpha: 255,
            }),
            "lightyellow" => Ok(RinkColor {
                rgb: rgb(255, 255, 224),
                alpha: 255,
            }),
            "lightblue" => Ok(RinkColor {
                rgb: rgb(173, 216, 230),
                alpha: 255,
            }),
            // light magenta does not exist
            "orchid" => Ok(RinkColor {
                rgb: rgb(218, 112, 214),
                alpha: 255,
            }),
            "lightcyan" => Ok(RinkColor {
                rgb: rgb(224, 255, 255),
                alpha: 255,
            }),
            "white" => Ok(RinkColor {
                rgb: rgb(225, 255, 255),
                alpha: 255,
            }),
            _ => {
                if color.len() == 7 && color.starts_with('#') {
                    parse_hex(color).map(|c| RinkColor {
                        rgb: rgb_from_slice(c),
                        alpha: 255,
                    })
                } else if let Some(stripped) = color.strip_prefix("rgb(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (alpha, rgb_values) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = alpha.parse() {
                            parse_rgb(rgb_values).map(|c| RinkColor {
                                rgb: rgb_from_slice(c),
                                alpha: a,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_rgb(color_values).map(|c| RinkColor {
                            rgb: rgb_from_slice(c),
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("rgba(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_rgb(rgb_values).map(|c| RinkColor {
                                rgb: rgb_from_slice(c),
                                alpha: (a * 255.0) as u16,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_rgb(color_values).map(|c| RinkColor {
                            rgb: rgb_from_slice(c),
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("hsl(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_hsl(rgb_values).map(|c| RinkColor {
                                rgb: rgb_from_slice(c),
                                alpha: (a * 255.0) as u16,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_hsl(color_values).map(|c| RinkColor {
                            rgb: rgb_from_slice(c),
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("hsla(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_hsl(rgb_values).map(|c| RinkColor {
                                rgb: rgb_from_slice(c),
                                alpha: (a * 255.0) as u16,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_hsl(color_values).map(|c| RinkColor {
                            rgb: rgb_from_slice(c),
                            alpha: 255,
                        })
                    }
                } else {
                    Err(ParseColorError)
                }
            }
        }
    }
}

const fn to_rgb(c: Color) -> u16x4 {
    match c {
        Color::Black => rgb(0, 0, 0),
        Color::DarkRed => rgb(255, 0, 0),
        Color::DarkGreen => rgb(0, 128, 0),
        Color::DarkYellow => rgb(255, 255, 0),
        Color::DarkBlue => rgb(0, 0, 255),
        Color::DarkMagenta => rgb(255, 0, 255),
        Color::DarkCyan => rgb(0, 255, 255),
        Color::DarkGrey => rgb(169, 169, 169),
        Color::Grey => rgb(128, 128, 128),
        Color::Red => rgb(255, 69, 0),
        Color::Green => rgb(144, 238, 144),
        Color::Yellow => rgb(255, 255, 224),
        Color::Blue => rgb(173, 216, 230),
        Color::Magenta => rgb(218, 112, 214),
        Color::Cyan => rgb(224, 255, 255),
        Color::White => rgb(255, 255, 255),
        Color::Rgb { r, g, b } => rgb(r as u16, g as u16, b as u16),
        Color::AnsiValue(idx) => match idx {
            16..=231 => {
                let v = idx - 16;
                // add 3 to round up
                let r = ((v as u16 / 36) * 255 + 3) / 5;
                let g = (((v as u16 % 36) / 6) * 255 + 3) / 5;
                let b = ((v as u16 % 6) * 255 + 3) / 5;
                rgb(r, g, b)
            }
            232..=255 => {
                let l = (idx - 232) / 24;
                u16x4::splat(l as u16)
            }
            // rink will never generate these colors, but they might be on the screen from another program
            _ => rgb(0, 0, 0),
        },
        Color::Reset => rgb(0, 0, 0),
    }
}

pub fn convert(mode: RenderingMode, c: u16x4) -> Color {
    match mode {
        crate::RenderingMode::BaseColors => {
            const COLORS: [(Color, u16x4); 16] = [
                (Color::Black, to_rgb(Color::Black)),
                (Color::DarkRed, to_rgb(Color::DarkRed)),
                (Color::DarkGreen, to_rgb(Color::DarkGreen)),
                (Color::DarkYellow, to_rgb(Color::DarkYellow)),
                (Color::DarkBlue, to_rgb(Color::DarkBlue)),
                (Color::DarkMagenta, to_rgb(Color::DarkMagenta)),
                (Color::DarkCyan, to_rgb(Color::DarkCyan)),
                (Color::DarkGrey, to_rgb(Color::DarkGrey)),
                (Color::Grey, to_rgb(Color::Grey)),
                (Color::Red, to_rgb(Color::Red)),
                (Color::Green, to_rgb(Color::Green)),
                (Color::Yellow, to_rgb(Color::Yellow)),
                (Color::Blue, to_rgb(Color::Blue)),
                (Color::Magenta, to_rgb(Color::Magenta)),
                (Color::Cyan, to_rgb(Color::Cyan)),
                (Color::White, to_rgb(Color::White)),
            ];

            // find the closest color based on the manhattan distance
            COLORS
                .iter()
                .min_by_key(|(_, rgb)| (c.max(*rgb) - c.min(*rgb)).wrapping_sum())
                .unwrap()
                .0
        }
        crate::RenderingMode::Rgb => {
            let mut rgb = [0; 4];
            c.write_to_slice_unaligned(&mut rgb);
            Color::Rgb {
                r: rgb[0] as u8,
                g: rgb[1] as u8,
                b: rgb[2] as u8,
            }
        }
        crate::RenderingMode::Ansi => {
            // 16-231: 6 × 6 × 6 color cube
            // 232-255: 23 step grayscale
            if c.extract(0) == c.extract(1) && c.extract(1) == c.extract(2) {
                let idx = 232 + (c.extract(0) * 23 / 255) as u8;
                Color::AnsiValue(idx)
            } else {
                let rgb = ((c * 5) / 255) * u16x4::new(36, 6, 1, 0);
                let idx = 16 + rgb.wrapping_sum();
                Color::AnsiValue(idx as u8)
            }
        }
    }
}

#[test]
fn rgb_to_ansi() {
    for idx in 17..=231 {
        let idxed = Color::AnsiValue(idx);
        let packed = to_rgb(idxed);
        let mut rgb = [0; 4];
        packed.write_to_slice_unaligned(&mut rgb);
        let converted = convert(RenderingMode::Ansi, packed);
        // gray scale colors have two equivelent repersentations
        if let Color::AnsiValue(i) = converted {
            if rgb[0] != rgb[1] || rgb[1] != rgb[2] {
                assert_eq!(idxed, converted);
            } else {
                assert!(i >= 232);
            }
        } else {
            panic!("color is not indexed")
        }
    }
    for idx in 232..=255 {
        let idxed = Color::AnsiValue(idx);
        let packed = to_rgb(idxed);
        let mut rgb = [0; 4];
        packed.write_to_slice_unaligned(&mut rgb);
        assert!(rgb[0] == rgb[1] && rgb[1] == rgb[2]);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RinkStyle {
    pub fg: Option<RinkColor>,
    pub bg: Option<RinkColor>,
    pub attributes: Attributes,
}

impl Default for RinkStyle {
    fn default() -> Self {
        Self {
            fg: Some(RinkColor {
                rgb: rgb(255, 255, 255),
                alpha: 255,
            }),
            bg: None,
            attributes: Attributes::default(),
        }
    }
}

impl RinkStyle {
    pub fn merge(mut self, other: RinkStyle) -> Self {
        self.fg = self.fg.or(other.fg);
        self.attributes.extend(other.attributes);
        self
    }

    pub fn add_attribute(mut self, attr: Attribute) -> Self {
        self.attributes.set(attr);
        self
    }

    pub fn remove_attribute(mut self, attr: Attribute) -> Self {
        self.attributes.unset(attr);
        self
    }
}

pub(crate) const fn rgb(r: u16, g: u16, b: u16) -> packed_simd::u16x4 {
    packed_simd::u16x4::new(r, g, b, 0)
}

pub(crate) fn rgb_from_slice(rgb: [u8; 3]) -> packed_simd::u16x4 {
    packed_simd::u16x4::new(rgb[0] as u16, rgb[1] as u16, rgb[2] as u16, 0)
}
