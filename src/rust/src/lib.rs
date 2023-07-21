use extendr_api::ellipsis::EllipsisIter;
use extendr_api::ellipsis::EllipsisValue;
use extendr_api::prelude::*;
use extendr_api::MissingArgId;

/// Return string `"Hello world!"` to R.
/// @export
#[extendr]
fn hello_world() -> &'static str {
    "Hello world!"
}

// only needed if return extendr_api::Error to R
// COMMENT incomplete conversion table, only handling case relevant for this example r-polars has a more detailed conversion
// Ideally conversion should be iso-morphic to allow round trip between extendr_api::Error and Robj
fn error_to_robj(err: Error) -> Robj {
    let err_msg = err.to_string();
    match err {
        Error::EvalError(robj) => robj,

        _ => format!("Extendr error: {:?}", err_msg).into(),
    }
}

// only need if return std::result::Result>Robj,Robj> to R
pub fn r_result_list<T, E>(result: std::result::Result<T, E>) -> list::List
where
    T: IntoRobj, //or Into<Robj>
    E: IntoRobj,
{
    match result {
        Ok(t) => list!(ok = t.into_robj(), err = NULL),
        Err(e) => list!(ok = NULL, err = e.into_robj()),
    }
    .as_list()
    .expect("internal error: cannot upcast a Robj being a list to a list")
}

// needed to catch actual error from promise, see trycatch_promise()
fn unpack_r_result_list(robj: Robj) -> std::result::Result<Robj, Robj> {
    let l = robj.as_list().expect("r_result_list: must be list");
    match (l.elt(0), l.elt(1)) {
        (Ok(ok), Ok(err)) if err.is_null() => Ok(ok),
        (Ok(ok), Ok(err)) if ok.is_null() => Err(err),
        (_, _) => panic!("r_result_list: must have two elements ok+err"),
    }
}

// needed to catch actual error from promise
fn trycatch_promise(p: Promise) -> Result<Robj> {
    let promise_r_result_list = eval_string_with_params(
        "tryCatch(list(ok=param.0,err=NULL),error= function(err) {list(ok=NULL,err=err)})",
        &[&p.into_robj()],
    )
    .expect("tryCatch never fails");
    unpack_r_result_list(promise_r_result_list).map_err(Error::EvalError)
}

// COMMENT: I GUESS MOST USERS JUST WANNER ITERATE OVER SYNATIC VALID NAMES AND PROMISES
#[derive(Debug)]
pub struct NamedPromise {
    name: Option<String>,
    promise: Promise,
}

// Implement NamePromiseIterator
#[derive(Clone)]
pub struct List2PromiseIter {
    ei: EllipsisIter,
    arg_idx: usize,
}
impl List2PromiseIter {
    pub fn new(ei: EllipsisIter) -> Self {
        List2PromiseIter { ei, arg_idx: 0 }
    }
}
impl From<EllipsisIter> for List2PromiseIter {
    fn from(ei: EllipsisIter) -> Self {
        Self::new(ei)
    }
}
impl Default for List2PromiseIter {
    fn default() -> Self {
        List2PromiseIter::new(EllipsisIter::new())
    }
}

/// Iterate according to rlang::list2 syntax rules over a ... args.
/// Item Ok a syntactic valid arg as NamedPromise
/// Iter Err a syntatic invalid arg as an error Value
/// any valid trailing arg is ignored as if it were not there. Hence it makes
/// no difference if there is a trialing comma or nor.
impl Iterator for List2PromiseIter {
    type Item = Result<NamedPromise>;
    fn next(&mut self) -> Option<Self::Item> {
        self.ei
            .next()
            .map(|ellipsis_iter_item| {
                self.arg_idx += 1;
                let opt_name: Option<String> =
                    ellipsis_iter_item.name.map(|sym| sym.as_str().to_string());

                // get promise or check if missing is valid
                match ellipsis_iter_item.value.to_promise() {
                    Some(promise) => Some(Ok(NamedPromise {
                        name: opt_name,
                        promise,
                    })),
                    None => {
                        //hmm no promise, was this the trailing arg? ...
                        match (self.ei.next(), &opt_name) {
                            // ... nope, more args to come!  This is illegal syntax, return Err
                            (Some(_), _) => Some(Err(Error::NonTrailingMissingArg(
                                opt_name
                                    .map(|name| MissingArgId::Name(name.as_str().into()))
                                    .unwrap_or_else(|| MissingArgId::Index(self.arg_idx)),
                            ))),

                            // ...trailing arg but it is named. That is not ok either
                            (None, Some(name)) => Some(Err(Error::Other(format!(
                                "trailing [{name}=] arg in ... was named but not defined"
                            )))),

                            // missing trailing unnamed arg, all good carry on
                            (None, None) => None,
                        }
                    }
                }
            })
            .flatten() //let iterator skip any trailing missing arg
    }
}

/// High Level imlpementations for List2PromiseIter. These Allow to get values
impl List2PromiseIter {
    /// Evaluate promises in R and return Ok vector of EllipsisValue if no errors
    /// if any first eval error immeditaly return Err the caught R error condition.
    pub fn trycatch_values(self) -> Result<Vec<EllipsisValue>> {
        self.map(|np_res: Result<NamedPromise>| {
            np_res.and_then(|np: NamedPromise| {
                trycatch_promise(np.promise).map(|ok_robj| EllipsisValue {
                    name: np.name,
                    value: ok_robj,
                })
            })
            //some instanciation of
        })
        .collect()
    }

    /// eval promises and return EllipisValue
    ///
    /// This method will instantly throw any eval error in R.
    /// BEWARE: INSTANTLY THROWN ERRORS CANNOT BE CAUGHT AGAIN.
    pub fn eval_values(self) -> Result<Vec<EllipsisValue>> {
        self.map(|np_res: Result<NamedPromise>| {
            np_res.and_then(|np: NamedPromise| {
                //some instanciation of
                np.promise.eval().map(|ok_robj| EllipsisValue {
                    name: np.name,
                    value: ok_robj,
                })
            })
        })
        .collect()
    }
}

///use example use Lit2PromiseIter in a for lopp
/// @export
#[extendr(use_try_from = true)]
fn iter_dots(#[ellipsis] dots: Ellipsis) {
    let _ = CheckMemRelease::new();
    for i in List2PromiseIter::new(dots.iter()) {
        match i {
            Ok(x) => {
                rprintln!("valid: {x:?}");
            }
            Err(Error::NonTrailingMissingArg(x)) => {
                rprintln!("oups ... only allow trailing missing args: {x:?}");
            }
            Err(x) => {
                rprintln!("another error very surprisingly happended {x:?}");
            }
        };
    }
}

/// @export
#[extendr(use_try_from = true)]
fn trycatch_dots(#[ellipsis] dots: Ellipsis) -> Result<List> {
    let _ = CheckMemRelease::new();
    let res: Result<Vec<EllipsisValue>> = List2PromiseIter::new(dots.iter()).trycatch_values();
    res.map(|x| List::from_pairs(x.into_iter()))
}

/// @export
#[extendr(use_try_from = true)]
fn trycatch_dots_result(#[ellipsis] dots: Ellipsis) -> List {
    let _ = CheckMemRelease::new();
    let res: Result<Vec<EllipsisValue>> = List2PromiseIter::new(dots.iter()).trycatch_values();
    let res = res
        .map(|x| List::from_pairs(x.into_iter()))
        .map_err(|err| error_to_robj(err));
    r_result_list(res)
}

/// @export
#[extendr(use_try_from = true)]
fn eval_dots(#[ellipsis] dots: Ellipsis) -> Result<List> {
    let _ = CheckMemRelease::new();
    let res: Result<Vec<EllipsisValue>> = List2PromiseIter::new(dots.iter()).eval_values();
    res.map(|x| List::from_pairs(x.into_iter()))
}

/// @export
#[extendr(use_try_from = true)]
fn collect_dots(#[ellipsis] dots: Ellipsis) -> Result<List> {
    let _ = CheckMemRelease::new();
    let res = dots.values();
    res.map(|x| List::from_pairs(x.into_iter()))
}

struct CheckMemRelease {}

impl CheckMemRelease {
    pub fn new() -> Self {
        CheckMemRelease {}
    }
}

impl Drop for CheckMemRelease {
    fn drop(&mut self) {
        rprintln!("CheckMemReleased: OK");
    }
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod helloextendr;
    fn iter_dots;
    fn trycatch_dots;
    fn trycatch_dots_result;
    fn eval_dots;
    fn collect_dots;
    fn hello_world;
}
