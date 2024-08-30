# Directives list

Tip: use a mapping from directive to phase for certain of the following directives. Clear that mapping on `reset_state()`.

* [x] Variable definition
  * [ ] Framework specific meta-data
* [x] Function definition
  * [ ] Framework specific meta-data (`[Bindable]` for example)
* [ ] Class definition
  * [ ] Handle the `[Bindable]` meta-data right after variables are declared
  * [ ] Handle the `[Embed]` meta-data.
* [ ] Enum definition
* [ ] Interface definition
* [ ] Type definition
* [ ] Namespace definition
  * [ ] Declares an alias to a namespace.
  * [ ] If right-hand side is a string literal, then declare namespace directly in Alpha phase instead of resolving the constant at Beta phase (in which case it is preceded by an UnresolvedEntity).
  * [ ] In constant resolution, if the constant is a String, then a declaration occurs; otherwise it it should be a Namespace constant.
  * [ ] Do not allow shadowing properties in base classes (`verifier.ensure_not_shadowing_definition(...)`)
* [x] Block
* [x] Labeled statement
* [x] If statement
* [x] Switch statement
* [x] Switch type statement
  * [ ] Verify case binding if any
* [x] Do statement
* [x] While statement
* [x] For statement
  * [ ] Verify variables if any
* [x] For..in statement
  * [ ] Verify variable if any
* [x] With statement
* [x] Try statement
  * [ ] Verify catch binding
* [x] Configuration directive
* [x] Import directive
* [x] Use namespace directive
* [x] Include directive
* [x] Normal configuration directive
* [x] Package concatenation directive
* [x] Directive injection