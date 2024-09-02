# Directives list

Tip: use a mapping from directive to phase for certain of the following directives. Clear that mapping on `reset_state()`.

* [x] Variable definition
  * [ ] Framework specific meta-data
* [x] Function definition
  * [ ] Framework specific meta-data (`[Bindable]` for example)
* [x] Class definition
  * [ ] Handle the `[Bindable]` meta-data right after variables are declared
  * [ ] Handle the `[Embed]` meta-data.
* [x] Enum definition
* [x] Interface definition
* [x] Type definition
* [x] Namespace definition
* [x] Block
* [x] Labeled statement
* [x] If statement
* [x] Switch statement
* [x] Switch type statement
  * [x] Verify case binding if any
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