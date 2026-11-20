#[allow(unused_imports)]
use origin::Private;
pub use origin::{First, Second as Renamed, nested as api};

pub fn marker() {}

pub use origin::{Generic, r#type::Keyword as r#match};
pub struct Lookalike;
pub mod cycle {
    pub use crate::cycle as again;
}
