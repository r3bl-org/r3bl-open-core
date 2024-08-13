/*
 *   Copyright (c) 2023 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use colorgrad::Gradient;
use r3bl_rs_utils_core::{RgbValue, TuiColor};
use rand::Rng;

/// # Arguments
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [TuiColor] objects representing the gradient.
pub fn generate_random_truecolor_gradient(steps: usize) -> Vec<TuiColor> {
    let mut rng = rand::thread_rng();

    let stops = vec![
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string(),
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string(),
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string(),
    ];

    generate_truecolor_gradient(&stops, steps)
}

/// # Arguments
/// * `stops` - A vector of hex strings representing the color stops.
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [TuiColor] objects representing the gradient.
pub fn generate_truecolor_gradient(stops: &[String], steps: usize) -> Vec<TuiColor> {
    let colors = stops.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    let result_gradient = colorgrad::GradientBuilder::new()
        .html_colors(&colors)
        .build::<colorgrad::LinearGradient>();

    type Number = f32;

    match result_gradient {
        Ok(gradient) => {
            let fractional_step: Number = (1 as Number) / steps as Number;

            let mut acc = vec![];

            for step_count in 0..steps {
                let color = gradient.at(fractional_step * step_count as Number);
                let color = color.to_rgba8();
                acc.push(TuiColor::Rgb(RgbValue {
                    red: color[0],
                    green: color[1],
                    blue: color[2],
                }));
            }

            acc
        }
        Err(_) => {
            // Gradient w/ 10 stops going from red to green to blue.
            [
                (255, 0, 0),
                (204, 51, 0),
                (153, 102, 0),
                (102, 153, 0),
                (51, 204, 0),
                (0, 255, 0),
                (0, 204, 51),
                (0, 153, 102),
                (0, 102, 153),
                (0, 51, 204),
            ]
            .iter()
            .map(|(red, green, blue)| {
                TuiColor::Rgb(RgbValue {
                    red: *red,
                    green: *green,
                    blue: *blue,
                })
            })
            .collect::<Vec<TuiColor>>()
        }
    }
}

#[cfg(test)]
mod tests {
    use r3bl_ansi_color::{AnsiStyledText, Style};
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_generate_random_truecolor_gradient() {
        let steps = 10;
        let result = generate_random_truecolor_gradient(steps);

        assert_eq2!(result.len(), steps);

        result
            .iter()
            .enumerate()
            .for_each(|(index, color)| match color {
                TuiColor::Rgb(c) => {
                    AnsiStyledText {
                        text: format!(
                            " {index}                                                   "
                        )
                        .as_str(),
                        style: &[Style::Background(r3bl_ansi_color::Color::Rgb(
                            c.red, c.green, c.blue,
                        ))],
                    }
                    .println();
                }
                _ => panic!("Unexpected color type"),
            });
    }

    #[test]
    fn test_generate_truecolor_gradient() {
        let stops = ["#ff0000", "#00ff00", "#0000ff"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let steps = 10;
        let result = generate_truecolor_gradient(&stops, steps);

        assert_eq2!(result.len(), steps);

        [
            (255, 0, 0),
            (204, 51, 0),
            (153, 102, 0),
            (102, 153, 0),
            (51, 204, 0),
            (0, 255, 0),
            (0, 204, 51),
            (0, 153, 102),
            (0, 102, 153),
            (0, 51, 204),
        ]
        .iter()
        .enumerate()
        .for_each(|(i, (red, green, blue))| {
            assert_eq2!(
                result[i],
                TuiColor::Rgb(RgbValue {
                    red: *red,
                    green: *green,
                    blue: *blue,
                })
            );
        });

        result
            .iter()
            .enumerate()
            .for_each(|(index, color)| match color {
                TuiColor::Rgb(c) => {
                    AnsiStyledText {
                        text: format!(
                            " {index}                                                   "
                        )
                        .as_str(),
                        style: &[Style::Background(r3bl_ansi_color::Color::Rgb(
                            c.red, c.green, c.blue,
                        ))],
                    }
                    .println();
                }
                _ => panic!("Unexpected color type"),
            });
    }
}
