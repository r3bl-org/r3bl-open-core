// INFO: For now, allow unused imports in this file, since it contains tests that use symbols.
// TODO: Remove this once the tests are moved to integration tests folder.
#![allow(unused_imports)]

use crate::tui::{Button, Select, Text};

/// Virtual DOM struct.
#[derive(Debug)]
pub struct VirtualDomElement {
  pub tag: String,
  pub value: Option<String>,
  pub children: Option<VirtualDomChildren>,
}
pub type VirtualDomChildren = Vec<VirtualDomElement>;

/// Virtual DOM builder.
impl VirtualDomElement {
  pub fn new(name: &str) -> VirtualDomElement {
    VirtualDomElement {
      tag: name.to_string(),
      value: None,
      children: None,
    }
  }

  pub fn value(
    mut self,
    value: &str,
  ) -> VirtualDomElement {
    self.value = Some(value.to_string());
    self
  }

  pub fn add_child(
    mut self,
    child: VirtualDomElement,
  ) -> VirtualDomElement {
    if let Some(ref mut children) = self.children {
      children.push(child);
    } else {
      self.children = Some(vec![child]);
    }
    self
  }

  pub fn set_children(
    mut self,
    children: VirtualDomChildren,
  ) -> VirtualDomElement {
    self.children = Some(children);
    self
  }

  pub fn move_to_string(self) -> String {
    format!(
      "{} {} {}",
      self.tag.to_string(),
      self.value.unwrap().to_string(),
      self
        .children
        .unwrap()
        .into_iter()
        .map(|c| format!("{}", c.move_to_string()))
        .collect::<Vec<String>>()
        .join(" ")
    )
  }
}

/// `Render` trait & `Component` type.
pub trait Render {
  fn render(&self) -> VirtualDomElement;
}
pub type Component = Box<dyn Render>;

/// `Screen` struct.
pub struct Screen {
  pub components: Vec<Component>,
}
impl Screen {
  pub fn render_all(&self) -> VirtualDomChildren {
    self
      .components
      .iter()
      .map(|component| component.render())
      .collect()
  }
}

#[test]
fn test_virtual_dom_react_prototype() {
  // Create a screen, add some components, and then render.
  let screen = Screen {
    components: vec![
      Box::new(Select {
        name: String::from("select"),
        options: vec!["option1".to_string(), "option2".to_string()],
      }),
      Box::new(Text {
        text: String::from("text"),
      }),
      Box::new(Button {
        width: 100,
        height: 100,
        label: String::from("button"),
      }),
    ],
  };
  println!("{:#?}", screen.render_all());
}
