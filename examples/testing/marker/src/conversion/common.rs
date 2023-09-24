#[repr(C)]
pub struct DefIdLayout {
    pub krate: u32,
    pub index: u32,
}

#[repr(C)]
pub struct BodyIdLayout {
    // Note: AFAIK rustc only loads bodies from the current crate, this allows
    // rustc to only store the index of the `DefId` and leave out the crate index.
    // Other drivers, will most likely require additional information, like the
    // crate id.
    pub owner: u32,
    pub index: u32,
}

/// Used as a target for [`Into`] implementations, note that it shouldn't be used as a transmute
/// target. Instead the specific ID layouts should be used.
pub struct DefIdInfo {
    pub index: u32,
    pub krate: u32,
}

#[repr(C)]
pub struct HirIdLayout {
    pub owner: u32,
    pub index: u32,
}

#[repr(C)]
pub struct ExpnIdLayout {
    pub krate: u32,
    pub index: u32,
}

#[macro_export]
macro_rules! transmute_id {
    ($t1:ty as $t2:ty = $e:expr) => {
        {
            assert_eq!(size_of::<$t1>(), size_of::<$t2>(), "the layout is invalid");
            // # Safety
            // The layout is validated with the `assert` above
            unsafe { transmute::<$t1, $t2>($e) }
        }
    };
}
