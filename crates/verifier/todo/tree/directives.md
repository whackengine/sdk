# Directives

## Defer

* [x] Statements are verified only after directives, in two different verification methods (one verification method for directives, and one pass verification method for statements). Block statements, with the right scopes, are entered recursively for directives.
* [x] Directives are always have a cache to prevent re-verification using the node mapping of SemanticHost; it may just be an invalidation entity when it does not matter, such as for an use namespace directive.
* [x] When at least one directive throws a defer error, the entire verification should reoccur next time.
* [x] Addition: the former explanations should be expanded such that deferred verification occurs in compilation unit level.

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

## Class initialiser method

Note that statements and static binding initializers within a class or enum block contribute code to the class initialiser method of AVM2, so control flow analysis should go from there rather than in the parent's initialiser (i.e. the package or top level).

## Class definitions

- [x] Alpha
- [x] Beta
  - [ ] 5. Handle the `[Embed]` meta-data if any (BUT DEFER ONLY AT THE FINAL STEP if necessary)
- [x] Omega
  - [ ] 3. (GUARD: do not double this step) Handle the `[Bindable]` meta-data but DEFER ONLY AT THE FINAL STEP if necessary.

## Function definitions

- [ ] `[Bindable]` for getter and setter