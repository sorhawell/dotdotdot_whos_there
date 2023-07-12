use extendr_api::prelude::*;

/// Return string `"Hello world!"` to R.
/// @export
#[extendr]
fn hello_world() -> &'static str {
    "Hello world!"
}

#[extendr(use_try_from = true)]
fn collect_dots(x: Robj, #[ellipsis] dots: Ellipsis, y: Robj) -> Result<List> {
    let dots = dots.values()?;
    let has_names = dots.iter().any(|v| v.name.is_some());

    let dots = if has_names {
        List::from_pairs(dots.into_iter())
    } else {
        List::from_values(dots.into_iter().map(|v| v.value))
    };

    Ok(list!(x = x, dots = dots, y = y))
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod helloextendr;
    fn collect_dots;
    fn hello_world;
}
