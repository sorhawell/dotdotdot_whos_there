reprex::reprex({

library(rlang)
library(helloextendr)

f_err = \() stop("boom")


#simple unwrap
unwrap = \(result) {
  if(!is.list(result) || !identical(names(result),c("ok","err"))) {
    stop("internal type error: cannot unwrap non result")
  }
  if(is.null(result$err)) {
    result$ok
  } else {
    stop(errorCondition(
      message = paste(capture.output(print(result$err)),collapse = "\n"),
      value = if(is.character(result$ok)) NULL else result$ok
    ))
  }
}

# normal.  -> ALL THE SAME
iter_dots(1,x=2,)
trycatch_dots(1,x=2,)
trycatch_dots_result(1,x=2,) |> unwrap()
eval_dots(1,x=2,)
collect_dots(1,x=2,)
list2(1,x=2,)


#invalid syntax before arg Error. -> NOT THE SAME
iter_dots(,f_err())
trycatch_dots(,f_err()) #A: panic on syntax Error
trycatch_dots_result(,f_err()) |> unwrap() #B: raise syntax error
eval_dots(,f_err())     # A: panic on syntax Error
collect_dots(,f_err())  # A: panic on syntax Error
list2(,f_err())  #B: raise syntax error


#arg Error before invalid syntax -> NOT THE SAME
iter_dots(f_err(),,)
trycatch_dots(f_err(),,)  #A: panic on arg Error
trycatch_dots_result(f_err(),,) |> unwrap()  #B: Raise Arg Error
eval_dots(f_err(),,)     #C: insta throw arg error, panic on generic extendr-error
collect_dots(f_err(),,)  #C: insta throw arg error, panic on generic extendr-error
list2(f_err(),,)         #B: Raise Arg Error

#TOO LATE TO CTACH f_err() from R
tryCatch(eval_dots(f_err()),error=\(err) as.character(err)) |> print()
tryCatch(collect_dots(f_err()),error=\(err) as.character(err)) |> print()


#error NamedMissingTrailingArg -> NOT ALL the same
iter_dots(1,x=)
trycatch_dots(1,x=) #A Panic on syntax error
trycatch_dots_result(1,x=) |> unwrap()  #B: Raise syntax Error
eval_dots(1,x=) #A: panic on syntax error
collect_dots(1,x=)  #C: ACCEPT TRAILING NAMED ARG  <-- THIS IS A BUG I THINK
list(1,x=) #B: Raise syntax Error

})
