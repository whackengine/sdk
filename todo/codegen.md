# Codegen

## External definitions

Definitions accompanied by `[Skywhack::External]` meta-data are only verified, and not compiled.

## Vector Data Type

The `Vector.<T>` data type translates to one of:

- [ ] `__AS3__.vec::Vector`
- [ ] `__AS3__.vec::Vector$double` for `T=Number`
- [ ] `__AS3__.vec::Vector$float` for `T=float`
- [ ] `__AS3__.vec::Vector$int` for `T=int`
- [ ] `__AS3__.vec::Vector$uint` for `T=uint`

## Bindable

See the [To Do List](sw.md) for Skywhack for the `[Bindable]` meta-data.

* [ ] Implement `[Bindable(...)]` at class definitions
* [ ] Implement `[Bindable(...)]` at variable definitions
* [ ] Implement `[Bindable(...)]` at setter definitions

## Embed

No notes as of yet.

## Conversions

* [ ] Visit conversion values in the node mapping carefully and travel until the topmost value of the conversion and pass it as a parameter to the node visitor rather than just directly taking the semantic entity from the node's mapping.

## Constant values

* [ ] Visit constant values in the node mapping before generating code for an expression. Constant values should yield a cheap AVM2 constant value.

## Call operator

* [ ] In JavaScript, emit either `call()`, `callproperty()`, or `callglobal()` for the call operator.

## Prototype

* [ ] Do not contribute the "prototype" property from a class object to the AVM2 bytecode. It is read implicitly in ActionCore.

## Non-null assertion operator

* [ ] For the `o!` operation, do not generate any assertion code, for efficiency (just used for type checking really).

## Asynchronous methods

Methods containing at least one `await` operator are asynchronous, in which case they return a `Promise`. In that case, the method body must be wrapped to wrap the JavaScript `Promise` object into an ActionScript 3 `Promise` object.