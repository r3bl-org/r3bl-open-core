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

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Ansi256GradientIndex {
    GrayscaleMediumGrayToWhite = 0, /* The remaining values are in incrementing
                                     * integer order. */
    DarkRedToDarkMagenta,
    RedToBrightPink,
    OrangeToNeonPink,
    LightYellowToWhite,
    MediumGreenToMediumBlue,
    GreenToBlue,
    LightGreenToLightBlue,
    LightLimeToLightMint,
    RustToPurple,
    OrangeToPink,
    LightOrangeToLightPurple,
    DarkOliveGreenToDarkLavender,
    OliveGreenToLightLavender,
    BackgroundDarkGreenToDarkBlue,
}

impl From<u8> for Ansi256GradientIndex {
    fn from(value: u8) -> Self {
        use Ansi256GradientIndex::*;
        match value {
            0..=14 => unsafe { std::mem::transmute::<u8, Ansi256GradientIndex>(value) },
            _ => GrayscaleMediumGrayToWhite, // Default fallback.
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ANSIColorArray(&'static [u8]);

/// More info:
/// <https://users.rust-lang.org/t/whats-the-idiomatic-way-to-store-array-of-global-constant-objects/14605>
pub static ANSI_256_GRADIENTS: [ANSIColorArray; 15] = [
    // For GrayscaleMediumGrayToWhite.
    ANSIColorArray(&[241, 242, 243, 244, 245, 247, 249, 251, 253, 255]),
    // For DarkRedToDarkMagenta.
    ANSIColorArray(&[124, 125, 126, 127, 128, 129]),
    // For RedToBrightPink.
    ANSIColorArray(&[160, 161, 162, 163, 164, 165]),
    // For OrangeToNeonPink.
    ANSIColorArray(&[202, 203, 204, 205, 206, 207]),
    // For LightYellowToWhite.
    ANSIColorArray(&[226, 227, 228, 229, 230, 231]),
    // For MediumGreenToMediumBlue.
    ANSIColorArray(&[34, 35, 36, 37, 38, 39]),
    // For GreenToBlue.
    ANSIColorArray(&[40, 41, 42, 43, 44, 45]),
    // For LightGreenToLightBlue.
    ANSIColorArray(&[118, 119, 120, 121, 122, 123]),
    // For LightLimeToLightMint.
    ANSIColorArray(&[190, 191, 192, 193, 194, 195]),
    // For RustToPurple.
    ANSIColorArray(&[130, 131, 132, 133, 134, 135]),
    // For OrangeToPink.
    ANSIColorArray(&[208, 209, 210, 211, 212, 213]),
    // For LightOrangeToLightPurple.
    ANSIColorArray(&[214, 215, 216, 217, 218, 219]),
    // For DarkOliveGreenToDarkLavender.
    ANSIColorArray(&[100, 101, 102, 103, 104, 105]),
    // For OliveGreenToLightLavender.
    ANSIColorArray(&[142, 143, 144, 145, 146, 147]),
    // For BackgroundDarkGreenToDarkBlue.
    ANSIColorArray(&[22, 23, 24, 25, 26, 27]),
];

#[must_use]
pub fn get_gradient_array_for(
    ansi_256_gradient_index: Ansi256GradientIndex,
) -> &'static [u8] {
    ANSI_256_GRADIENTS[ansi_256_gradient_index as usize].0
}

#[cfg(test)]
mod ansi_256_gradients_test {
    use super::*;
    use crate::{assert_eq2, LossyConvertToByte};

    #[test]
    fn test_all() {
        for (index, gradient) in ANSI_256_GRADIENTS.iter().enumerate() {
            let gradient_index = Ansi256GradientIndex::from(index.to_u8_lossy());
            assert_eq2!(gradient.0, get_gradient_array_for(gradient_index));
        }
    }
}
