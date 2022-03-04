use crate::tui::{Render, VirtualDomElement};

/// `Select` component that implements `Render` trait.
pub struct Select {
  pub name: String,
  pub options: Vec<String>,
}

impl Render for Select {
  fn render(&self) -> VirtualDomElement {
    let options_to_vdom = self
      .options
      .iter()
      .map(|option| VirtualDomElement::new("option").value(option))
      .collect::<Vec<VirtualDomElement>>();
    VirtualDomElement::new("select").set_children(options_to_vdom)
  }
}
