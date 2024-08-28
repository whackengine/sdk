# Statements

## Return statement

* [ ] If the surrounding method's signature is unresolved, let the return statement be able to return anything, as it will be handled later in the FunctionCommon control flow analysis.
* [ ] If the surrounding method returns `Promise.<T>`
  * [ ] If a value is specified
    * [ ] Implicitly coerce the value to `T`.
  * [ ] If no value is specified
    * [ ] If `T` is not `void` or `*`
      * [ ] Report a verify error
* [ ] Otherwise
  * Let E be the result type.
  * [ ] If a value is specified
    * [ ] Implicitly coerce the value to `E`.
  * [ ] If no value is specified
    * [ ] If `E` is not `void` or `*`
      * [ ] Report a verify error

## Switch type statement

* [ ] Reuse scope from block for the parenthesized binding in cases.
* [ ] Handle any name conflict for the parenthesized binding in cases.

## Try statement

* [ ] Reuse scope from block for the parenthesized binding in catch clauses.
* [ ] Handle any name conflict for the parenthesized binding in catch clauses.