# Verifier

## External definitions

In external definitions (`[Whack::External(...)]` or these contained within a class that has this meta-data), the verifier puts some restrictions, such as requiring only `native` or `abstract` methods, empty package and global initialization code, and variable bindings may only be assigned a constant.

## Locations

Do not forget to set source locations of entities such as classes and variables.

* [ ] Aliases
* [ ] Classes
* [ ] Enumerations
* [ ] Interfaces
* [ ] Type parameter types
* [ ] Variable slots
* [ ] Virtual slots
* [ ] Method slots

## ASDoc

* [ ] Set ASDoc comments properly, except for aliases.

## Meta-data

* [ ] Set meta-data properly, except for aliases.
* [ ] Handle Whack `[Bindable]`
* [ ] Handle Whack `[Embed]`
* [ ] Handle Whack `[Event]`

### @copy

* [ ] Correct anchor links from original source path to substitution source path.

### @inheritDoc

* [ ] Correct anchor links from original source path to substitution source path.

## Namespaces

* [ ] Throw a verify error if the namespace's local name conflicts with that of a configuration namespace in `host.config_constants()`.
* [ ] Combining the `static protected` modifiers in an annotatable directive indicates a `SystemNamespaceKind::StaticProtected` system namespace.
* [ ] Set ASDoc comments properly for explicit or user namespaces.
* [ ] `namespace ns1;` creates an `internal` system namespace belonging to a hardcoded package created in the fly, rather than an `UserNamespace`.
* [ ] `namespace ns1 = "...";` creates an user namespace (`UserNamespace`; not `ExplicitNamespace`).

## Packages

* [ ] The topmost scope of a package is an activation with `set_is_package_initialization(true)`, from which the package scope is subsequent.

## Activations

* [ ] Set `this()` properly in activations. For class static methods, global initialization code, and package initialization code, `this()` should always be `None`.

## Global initialization code

* [ ] For global initialization code, the topmost activation must set `public_ns()` and `internal_ns()`, for use with reserved namespace expressions and attribute combinations.

## Methods

* [ ] Set `is_async()`, `is_generator()`, and `is_constructor()` properly in method slots.
* [ ] Auto wrap asynchronous method's result type from signature into `Promise` if not already a `Promise`.

## Property access

* [ ] For dot and brackets operators, after filtering for shadowing package names
  * At first check if the base is a reference to a type (`PackageReferenceValue` or `ScopeReferenceValue` with a `property` that matches a type)
    * Lookup for property in that type first
  * Finally, if the first check returned `Ok(None)` or did not occur, lookup for property in the reference's resulting data type (e.g. `Class`).

## Open namespaces

Open namespaces properly everywhere.

* [ ] Package definitions opens the package's `internal`
* [ ] Class definition opens its `private`, `protected`, `static protected`, and also the inherited classes's `protected` and `static protected`.
* [ ] Enum definition opens its `private`.

## Parents

* [ ] Set parents correctly in all definitions.
  * [ ] Enclosing scope, type, or package, for example.

## Attributes

* [ ] Restrict definitions at package block to be either `public` or `internal`.
* [ ] Restrict definitions at top-level to be `internal`.
* [ ] Definitions at the top-level of a class may be in any namespace.
* [ ] Restrict user-defined namespaces to be used only at the top-level of class definitions.

## Getters and setters

* [ ] Invoke `set_of_virtual_slot()` properly.

## Inline constants

* [x] Expand inline constants

## Enums

* [ ] Define member slots with a `T!` non-null data type instead of `T` as-is.
* [ ] Perform mapping from member String to Number and from String to member variable slot

## Activations

* [ ] In most `FunctionCommon`, `this` is set to always be of the `*` data type.

## Signatures

* [ ] Restrict the rest parameter to be `Array.<T>`. If it is untyped, it defaults to `[*]`.
* [ ] Restrict the rest parameter's data type to not be a non-nullable layer over `Array.<T>`.

## Initial scope

* [ ] The initial scope of a package opens the `AS3` namespace (when at AS3 mode) and imports the top-level package.
* [ ] The initial scope of a program's directive sequence opens the `AS3` namespace (when at AS3 mode) and imports the top-level package.

## Options classes

* [ ] Mark them implicitly final.
* [ ] Restrict them to extend only Object.
* [ ] Restrain all fields to be writable.

## Constructors

* [ ] Require a non default constructor (a constructor with a non-empty parameter list) to be invoked from descending constructors.

## Hoisting

* [ ] If block-scoping is on, variables do not hoist to the activation (but functions still do).

## Return statement

* [ ] Report the following errors for the `return` statement used in global or package initialization code. To detect this:
  * [ ] Make sure to place either `set_is_global_initialization(true)` or `set_is_package_initialization(true)` in the activation.

```plain
Error: The return statement cannot be used in global initialization code.
Error: The return statement cannot be used in package initialization code.
```

## Top level

* [ ] The topmost scope of top level code is an activation with `set_is_global_initialization(true)`.

## Classes and enumerations

* [ ] Do not allow destructuring in variable bindings belonging to package, class, or enum top level declarations. Only simple identifiers (without non-null operator) may be used as variable bindings in these places.

# Virtual slots

* [ ] Delegate `[Bindable]` meta-data's semantic from setter or getter to the virtual slot they belong to.