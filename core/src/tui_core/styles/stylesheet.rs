/*
 *   Copyright (c) 2022 R3BL LLC
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

use crate::*;

#[derive(Default, Debug, Clone)]
pub struct Stylesheet {
  pub styles: Vec<Style>,
}

impl Stylesheet {
  pub fn new() -> Self { Self::default() }

  pub fn add_style(&mut self, style: Style) -> CommonResult<()> {
    throws!({
      if style.id.is_empty() {
        return CommonError::new_err_with_only_msg("Style id cannot be empty");
      }
      self.styles.push(style);
    });
  }

  pub fn add_styles(&mut self, styles: Vec<Style>) -> CommonResult<()> {
    throws!({
      for style in styles {
        self.add_style(style)?;
      }
    });
  }

  pub fn find_style_by_id(&self, id: &str) -> Option<Style> {
    self.styles.iter().find(|style| style.id == id).cloned()
  }

  /// Returns [None] if no style in `ids` [Vec] is found.
  pub fn find_styles_by_ids(&self, ids: Vec<&str>) -> Option<Vec<Style>> {
    let mut styles = Vec::new();

    for id in ids {
      if let Some(style) = self.find_style_by_id(id) {
        styles.push(style.clone());
      }
    }

    if styles.is_empty() {
      None
    } else {
      Some(styles)
    }
  }

  pub fn compute(styles: Option<Vec<Style>>) -> Option<Style> {
    if let Some(styles) = styles {
      let mut computed = Style::default();
      styles.iter().for_each(|style| computed += style);
      Some(computed)
    } else {
      None
    }
  }
}

/// Macro to make building [Stylesheet] easy.
///
/// Here's an example.
/// ```ignore
/// fn create_stylesheet() -> CommonResult<Stylesheet> {
///   throws_with_return!({
///     stylesheet! {
///         style! {
///           id: style1
///           margin: 1
///           color_bg: Color::Rgb { r: 55, g: 55, b: 248 }
///         },
///         vec![
///             style! {
///                 id: style1
///                 margin: 1
///                 color_bg: Color::Rgb { r: 55, g: 55, b: 248 }
///             },
///             style! {
///                 id: style2
///                 margin: 1
///                 color_bg: Color::Rgb { r: 85, g: 85, b: 255 }
///             },
///         ]
///     }
///   })
/// }
/// ```
#[macro_export]
macro_rules! stylesheet {
  ($($style:expr),*) => {
    {
      let mut style_sheet = Stylesheet::new();
      $(
        ($style).add_to_style_sheet(&mut style_sheet)?;
      )*
      style_sheet
    }
  };
}

pub trait AddStyle {
  fn add_to_style_sheet(self, stylesheet: &mut Stylesheet) -> CommonResult<()>;
}

impl AddStyle for Style {
  fn add_to_style_sheet(self, stylesheet: &mut Stylesheet) -> CommonResult<()> {
    stylesheet.add_style(self)
  }
}

impl AddStyle for Vec<Style> {
  fn add_to_style_sheet(self, stylesheet: &mut Stylesheet) -> CommonResult<()> {
    stylesheet.add_styles(self)
  }
}
