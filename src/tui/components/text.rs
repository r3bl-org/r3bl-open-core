use crate::tui::{Render, VDOMElement};

/// `Text` component that implements `Render` trait.
pub struct Text {
  pub text: String,
}

impl Render for Text {
  fn render(&self) -> VDOMElement {
    VDOMElement::new("text").value(&self.text)
  }
}
