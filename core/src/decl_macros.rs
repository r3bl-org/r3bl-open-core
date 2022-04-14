/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.

 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at

 *   http://www.apache.org/licenses/LICENSE-2.0

 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
*/

#[macro_export]
macro_rules! debug {
  ($i:ident) => {
    println!(
      "{} {} = {}",
      r3bl_rs_utils::style_error("▶"),
      r3bl_rs_utils::style_prompt(stringify!($i)),
      r3bl_rs_utils::style_dimmed(&format!("{:#?}", $i))
    );
  };
}

/// Declarative macro to generate the API call functions. This adds the following:
/// - `make_request()` async function to call the API.
/// - `to_string()` function to stringify the struct to JSON.
/// - impl `Display` trait to for the struct using `to_string()` above.
#[macro_export]
macro_rules! make_api_call_for {
  ($STRUCT_NAME:ident at $ENDPOINT:ident) => {
    pub async fn make_request() -> Result<$STRUCT_NAME, Box<dyn Error>> {
      let res = reqwest::get($ENDPOINT).await?;
      let res_text = res.text().await?;
      let res_json: $STRUCT_NAME = serde_json::from_str(&res_text)?;
      Ok(res_json)
    }

    impl $STRUCT_NAME {
      pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
      }
    }

    impl Display for $STRUCT_NAME {
      fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
      ) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
      }
    }
  };
}
