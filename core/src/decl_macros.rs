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

/// Declarative macro to surround the given block with a call to [`tokio::spawn`]. This is
/// useful for spawning a task that will run in the background from a function that is NOT
/// async.
///
/// # Examples:
///
/// Your block can be sync and `foo` is not async.
///
/// ```no_run
/// pub fn foo() {
///   fire_and_forget!(
///     { println!("Hello"); }
///   );
/// }
/// ```
///
/// Your block can be async and `foo` is still not async.
///
/// ```no_run
/// pub fn foo() {
///   fire_and_forget!(
///      let fake_data = fake_contact_data_api()
///      .await
///      .unwrap_or_else(|_| FakeContactData {
///        name: "Foo Bar".to_string(),
///        phone_h: "123-456-7890".to_string(),
///        email_u: "foo".to_string(),
///        email_d: "bar.com".to_string(),
///        ..FakeContactData::default()
///      });
///   );
/// }
#[macro_export]
macro_rules! fire_and_forget {
  ($block:block) => {
     return tokio::spawn(async move { $block });
  };
}

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
  ($IDENT:ident at $ENDPOINT:ident) => {
    pub async fn make_request() -> Result<$IDENT, Box<dyn Error>> {
      let res = reqwest::get($ENDPOINT).await?;
      let res_text = res.text().await?;
      let res_json: $IDENT = serde_json::from_str(&res_text)?;
      Ok(res_json)
    }

    impl $IDENT {
      pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
      }
    }

    impl Display for $IDENT {
      fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
      ) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
      }
    }
  };
}
