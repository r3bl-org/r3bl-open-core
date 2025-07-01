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

//! True color (RGB) gradient generation functionality.
//!
//! This module provides functions for generating smooth color gradients using
//! true color (24-bit RGB) values:
//! - `generate_truecolor_gradient()` - Creates gradients from hex color stops
//! - `generate_random_truecolor_gradient()` - Creates gradients with random colors
//!
//! Uses the `colorgrad` crate for smooth interpolation between color stops.
//! Previously located in `color_wheel_core/truecolor_gradient.rs`.

use colorgrad::Gradient;
use rand::{rngs::ThreadRng, Rng};

use crate::{config::sizing::{StringHexColor, VecSteps},
            tui_color};

/// # Arguments
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [`crate::TuiColor`] objects representing the gradient.
#[must_use]
pub fn generate_random_truecolor_gradient(steps: u8) -> VecSteps {
    let random_stops = [
        random_color::generate(),
        random_color::generate(),
        random_color::generate(),
    ];

    generate_truecolor_gradient(&random_stops, steps)
}

/// # Arguments
/// * `stops` - A vector of hex strings representing the color stops.
/// * `steps` - The number of steps to take between each color stop.
///
/// # Returns
/// A vector of [`crate::TuiColor`] objects representing the gradient.
#[must_use]
pub fn generate_truecolor_gradient(stops: &[StringHexColor], num_steps: u8) -> VecSteps {
    type Number = f32;

    let result_gradient = colorgrad::GradientBuilder::new()
        .html_colors(stops)
        .build::<colorgrad::LinearGradient>();

    match result_gradient {
        Ok(gradient) => {
            let fractional_step: Number = 1.0 / Number::from(num_steps);

            // Create an acc with the same capacity as the number of steps. And pre-fill
            // it with black.
            let mut acc = VecSteps::new();

            for step_count in 0..num_steps {
                let color = gradient.at(fractional_step * Number::from(step_count));
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

mod random_color {
    use super::*;

    pub fn generate() -> StringHexColor {
        let mut rng: ThreadRng = rand::rng();

        let random_color = colorgrad::Color::new(
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
            rng.random_range(0.0..1.0),
            1.0,
        );

        color_to_hex_string(&random_color)
    }

    /// Copied from [`colorgrad::Color::to_hex_string`], and modified to return a
    /// [`StringHexColor`] instead of a [String].
    pub fn color_to_hex_string(color: &colorgrad::Color) -> StringHexColor {
        use std::fmt::Write as _;

        let [r, g, b, a] = color.to_rgba8();

        let mut acc = StringHexColor::new();
        if a < 255 {
            _ = write!(acc, "#{r:02x}{g:02x}{b:02x}{a:02x}");
        } else {
            _ = write!(acc, "#{r:02x}{g:02x}{b:02x}");
        }

        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, ast, new_style, usize, TuiColor};

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
                    ast(
                        format!(
                            " {index}                                                   "
                        ),
                        new_style!(
                            color_bg: {tui_color!(c.red, c.green, c.blue)}
                        ),
                    )
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
                    ast(
                        format!(
                            " {index}                                                   "
                        ),
                        new_style!(
                            color_bg: {tui_color!(c.red, c.green, c.blue)}
                        ),
                    )
                    .println();
                }
                _ => panic!("Unexpected color type"),
            });
    }
}
