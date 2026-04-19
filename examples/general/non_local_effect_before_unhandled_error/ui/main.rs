#![allow(unused_must_use)]

use std::env::VarError;

fn main() {}

// --- Functions with non-local effects before error return ---

pub fn non_local_effect_deref_assign(flag: &mut bool) -> Result<(), VarError> {
    *flag = true;
    Err(VarError::NotPresent)
}

pub fn non_local_effect_call_with_mut_ref(xs: &mut Vec<u32>) -> Result<(), VarError> {
    xs.push(0);
    Err(VarError::NotPresent)
}

// --- A function without non-local effects ---

pub fn no_non_local_effect(x: u32) -> Result<u32, VarError> {
    if x == 0 {
        return Err(VarError::NotPresent);
    }
    Ok(x)
}

// --- Callers that DO NOT handle errors (should warn) ---

pub fn caller_ignore_let_underscore(flag: &mut bool) {
    // Unhandled: the result is dropped.
    let _ = non_local_effect_deref_assign(flag);
}

pub fn caller_ignore_semicolon(xs: &mut Vec<u32>) {
    // Unhandled: the result is dropped.
    non_local_effect_call_with_mut_ref(xs);
}

pub fn caller_partial_handle(flag: &mut bool, should_return: bool) -> Result<(), VarError> {
    let result = non_local_effect_deref_assign(flag);
    if should_return {
        // The Ok branch returns early, dropping `result` without handling its error.
        return Ok(());
    }
    result
}

// --- Callers that DO handle errors (should NOT warn) ---

pub fn caller_question_mark(flag: &mut bool) -> Result<(), VarError> {
    non_local_effect_deref_assign(flag)?;
    Ok(())
}

pub fn caller_return_directly(flag: &mut bool) -> Result<(), VarError> {
    non_local_effect_deref_assign(flag)
}

pub fn caller_unwrap(flag: &mut bool) {
    non_local_effect_deref_assign(flag).unwrap();
}

pub fn caller_expect(flag: &mut bool) {
    non_local_effect_deref_assign(flag).expect("must succeed");
}

pub fn caller_match_panic(flag: &mut bool) {
    match non_local_effect_deref_assign(flag) {
        Ok(_) => {}
        Err(_) => panic!("bad"),
    }
}

pub fn caller_assign_then_return(flag: &mut bool) -> Result<(), VarError> {
    let result = non_local_effect_deref_assign(flag);
    result
}

// --- Call to function without non-local effects: never warns ---

pub fn caller_ignore_call_with_no_non_local_effect(x: u32) {
    let _ = no_non_local_effect(x);
}

// --- fmt::Result is excluded from non-local effect tracking ---

pub mod fmt_result_test {
    use std::fmt::{self, Write};

    pub fn fmt_write(buffer: &mut String) -> fmt::Result {
        buffer.write_str("hello")?;
        Err(fmt::Error)
    }

    pub fn caller_ignores_fmt(buffer: &mut String) {
        // Should not warn: `fmt_write` returns `fmt::Result`, which is excluded from tracking.
        let _ = fmt_write(buffer);
    }
}

// --- Macro-originated effects are NOT treated as non-local effects ---
//
// The only "mutations" inside this function are internal to the `vec!` macro expansion,
// which the non-local effect detection should skip. The function therefore should not be tracked
// as having non-local effects, and the call in `caller_ignore_macro_non_local_effect` should not
// be flagged.

pub fn only_macro_effect(x: u32) -> Result<Vec<u32>, VarError> {
    let v = vec![x];
    if x == 0 {
        return Err(VarError::NotPresent);
    }
    Ok(v)
}

pub fn caller_ignore_macro_non_local_effect(x: u32) {
    let _ = only_macro_effect(x);
}

// --- Interprocedural: caller of a transitive function with non-local effects is NOT flagged ---

pub mod transitive {
    use std::env::VarError;

    pub fn direct_non_local_effect(flag: &mut bool) -> Result<(), VarError> {
        *flag = true;
        Err(VarError::NotPresent)
    }

    pub fn passthrough(flag: &mut bool) -> Result<(), VarError> {
        // `passthrough` does not itself perform a non-local effect (the mut-ref argument just
        // flows into `direct_non_local_effect`), so it should not be tracked as having non-local
        // effects.
        direct_non_local_effect(flag)
    }

    pub fn caller_of_passthrough(flag: &mut bool) {
        // Should not warn at the call to `passthrough` since it does not itself have non-local
        // effects.
        let _ = passthrough(flag);
    }
}
