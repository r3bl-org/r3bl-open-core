/*
 *   Copyright (c) 2023-2025 R3BL LLC
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
use rand::Rng;

use crate::{config::sizing::{StringHexColor, VecSteps},
            tui_color};

/// # Arguments
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [crate::TuiColor] objects representing the gradient.
pub fn generate_random_truecolor_gradient(steps: u8) -> VecSteps {
    let mut rng = rand::thread_rng();

    let stops = [
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string()
        .into(),
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string()
        .into(),
        colorgrad::Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            1.0,
        )
        .to_hex_string()
        .into(),
    ];

    generate_truecolor_gradient(&stops, steps)
}

/// # Arguments
/// * `stops` - A vector of hex strings representing the color stops.
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [crate::TuiColor] objects representing the gradient.
pub fn generate_truecolor_gradient(stops: &[StringHexColor], steps: u8) -> VecSteps {
    let result_gradient = colorgrad::GradientBuilder::new()
        .html_colors(stops)
        .build::<colorgrad::LinearGradient>();

    type Number = f32;

    match result_gradient {
        Ok(gradient) => {
            let fractional_step: Number = (1 as Number) / steps as Number;

            // Create an acc with the same capacity as the number of steps. And pre-fill
            // it with black.
            let mut acc = VecSteps::new();

            for step_count in 0..steps {
                let color = gradient.at(fractional_step * step_count as Number);
                let color = color.to_rgba8();
                acc.push(tui_color!(color[0], color[1], color[2]));
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
            .map(|(red, green, blue)| tui_color!(*red, *green, *blue))
            .collect::<VecSteps>()
        }
    }
}

#[cfg(test)]
mod tests {
    use r3bl_ansi_color::{AnsiStyledText, Style};

    use super::*;
    use crate::{TuiColor, assert_eq2, usize};

    #[test]
    fn test_generate_random_truecolor_gradient() {
        let steps = 10;
        let result = generate_random_truecolor_gradient(steps);

        assert_eq2!(result.len(), usize(steps));

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
        let stops = ["#ff0000".into(), "#00ff00".into(), "#0000ff".into()];
        let steps = 10;
        let result = generate_truecolor_gradient(&stops, steps);

        assert_eq2!(result.len(), usize(steps));

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
            assert_eq2!(result[i], tui_color!(*red, *green, *blue));
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
