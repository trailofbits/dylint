pub struct First;
pub struct Second;
pub struct Private;
pub mod nested {
    pub struct Third;
}

pub struct Generic<T>(pub T);
pub struct Lookalike;
pub mod r#type {
    pub struct Keyword;
}
