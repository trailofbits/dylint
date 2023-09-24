use std::marker::PhantomData;

use bumpalo::Bump;

pub struct Storage<'ast> {
    /// The `'ast` lifetime is the lifetime of the `buffer` field.
    ///
    /// Having it as an explicit parameter allows us to later add fields to cache values.
    _lifetime: PhantomData<&'ast ()>,
    buffer: Bump,
}

impl<'ast> Default for Storage<'ast> {
    fn default() -> Self {
        Self {
            _lifetime: PhantomData,
            buffer: Bump::new(),
        }
    }
}

impl<'ast> Storage<'ast> {
    #[must_use]
    pub fn alloc<T>(&'ast self, t: T) -> &'ast T {
        self.buffer.alloc(t)
    }

    #[must_use]
    pub fn alloc_slice<T, I>(&'ast self, iter: I) -> &'ast [T]
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.buffer.alloc_slice_fill_iter(iter)
    }

    #[must_use]
    pub fn alloc_str(&'ast self, value: &str) -> &'ast str {
        self.buffer.alloc_str(value)
    }
}
