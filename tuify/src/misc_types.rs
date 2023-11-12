use std::{ops::{AddAssign, Deref, DerefMut}};

pub mod global_constants {

    pub const SPACER: &str = " ";

}
pub use global_constants::*;

pub mod list_of {
    use get_size::GetSize;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[macro_export]
    macro_rules! list {
        (
            $($item: expr),*
            $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
        ) => {
            {
                #[allow(unused_mut)]
                let mut it = List::new();
                $(
                    it.items.push($item);
                )*
                it
            }
        };
    }

    /// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for [List] of
    /// `T`. Where `T` is any number of types in the tui crate.
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, GetSize)]
    pub struct List<T> {
        pub items: Vec<T>,
    }

    impl<T> List<T> {
        pub fn with_capacity(size: usize) -> Self {
            Self {
                items: Vec::with_capacity(size),
            }
        }

        pub fn new() -> Self { Self { items: Vec::new() } }
    }

    /// Add (other) item to list (self).
    impl<T> AddAssign<T> for List<T> {
        fn add_assign(&mut self, other_item: T) { self.push(other_item); }
    }

    /// Add (other) list to list (self).
    impl<T> AddAssign<List<T>> for List<T> {
        fn add_assign(&mut self, other_list: List<T>) { self.extend(other_list.items); }
    }

    /// Add (other) vec to list (self).
    impl<T> AddAssign<Vec<T>> for List<T> {
        fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
    }

    impl<T> From<List<T>> for Vec<T> {
        fn from(list: List<T>) -> Self { list.items }
    }

    impl<T> From<Vec<T>> for List<T> {
        fn from(other: Vec<T>) -> Self { Self { items: other } }
    }

    impl<T> Deref for List<T> {
        type Target = Vec<T>;
        fn deref(&self) -> &Self::Target { &self.items }
    }

    impl<T> DerefMut for List<T> {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.items }
    }
}
pub use list_of::*;
