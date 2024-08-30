# Directives

## Defer

* [ ] Statements are verified only after directives, in two different verification methods (one verification method for directives, and one pass verification method for statements). Block statements, with the right scopes, are entered recursively for directives.
* [ ] Directives are always have a cache to prevent re-verification using the node mapping of SemanticHost; it may just be an invalidation entity when it does not matter, such as for an use namespace directive.
* [ ] When at least one directive throws a defer error, the entire verification should reoccur next time.
* [ ] Addition: the former explanations should be expanded such that deferred verification occurs in compilation unit level.

## Scopes

Set scopes carefully within directive sequences. Sometimes inherit and enter; sometimes just overwrite it.

## Across compilation units

Across all compilation units, directives should be verified first, from a package to a class or method, and from a class to a method or property. After all directives are solved in these ranges, statements may be verified in one pass.

## Directives versus statements

The `DirectiveSubverifier::verify_directive()` method will verify a directive, for certain directives and the block statement, their subdirectives until a limit (for example, from class goes until methods, and from a block statement goes until subdirectives).

* `DirectiveSubverifier::verify_directives` will verify a list of directives and, in case it found any deferred part, it returns `Err` (but all directives are guaranteed to be have been verified).

The `StatementSubverifier::verify_statement()` method will verify a statement or all substatements from a directive such as a class or function definition. It does not throw a defer error; anything that defers will result into a verify error.

* `StatementSubverifier::verify_statements()` will verify a list of statements using `StatementSubverifier::verify_statement()`.

## Variable definitions

Procedure:

* [x] Alpha
* [x] Beta
* [x] Delta
* [x] Epsilon
  * [ ] Handle the `[Bindable]` meta-data for simple identifier patterns
  * [ ] Handle the `[Embed]` meta-data for simple identifier patterns
* [x] Omega

## Inheritance

* [ ] For classes and interfaces, right after the phase in which the inheritance is solved, ensure the inheritance is not circular (an inherited type must not be equals to or a subtype of the inheritor type) by reporting a verify error in such case.
* [ ] For definitions within classes and interfaces, ensure they either override a method or do not redefine a previously defined property.

## Class initialiser method

Note that statements and static binding initializers within a class or enum block contribute code to the class initialiser method of AVM2, so control flow analysis should go from there rather than in the parent's initialiser (i.e. the package or top level).

## Class definitions

* [ ] Assign ASDoc
* [ ] Assign meta-data
* [ ] Assign location
* [ ] Report a verify error for non overriden abstract methods
* [ ] If the base class contains a non-empty constructor, the subclass must define a constructor.
* [ ] Assign every `[Event]` semantics to the class
* [ ] Handle the `[Bindable]` meta-data right after variables are declared
* [ ] Handle the `[Embed]` meta-data.
* [ ] Assign attributes correctly (`static`, `dynamic`, `abstract`, and `final`)
* [ ] Mark unused

- Remember: `[SW::External]` (transitive)

- [ ] Alpha
  - [ ] 1. Attempt to define the class partially; or fail if a conflict occurs, therefore ignoring this class definition.
  - [ ] 2. Set parent
  - [ ] 3. Check if the `[Options]` meta-data is specified, therefore calling `set_is_options_class(true)`
- [ ] Beta
  - [ ] 1.
- [ ] Delta
  - [ ] 1. If `is_options_class()` is true and the class is not a direct subclass of `Object`, report a verify error.
- [ ] Epsilon
  - [ ] 1.
- [ ] Eta
  - [ ] 1.
- [ ] Omega
  - [ ] 1.

## Enum definitions

- Remember: for alpha, process defining constants and mark them as in the finished phase.

## Interface definitions

* [ ] Assign ASDoc
* [ ] Assign meta-data
* [ ] Assign location
* [ ] Assign every `[Event]` semantics to the interface
* [ ] Mark unused
* [ ] For the interface block, verify only top-level function definitions

- Remember: `[SW::External]`

## Function definitions

- [ ] `[Bindable]` for getter and setter