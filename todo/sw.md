# Whack

## Bindable

The `[Bindable]` meta-data is used to indicate that either a property or all properties of a class trigger a `PropertyChangeEvent` after write. The event is only dispatched if `newValue !== prevValue` (notice the strict `!==` operator).

The `[Bindable]` meta-data may be in one of the forms:

```
[Bindable]
[Bindable("eventName")]
[Bindable(event="eventName")]
```

If the event name is omitted, it defaults to `"propertyChange"`.

A setter may contain the `[Bindable]` meta-data, behaving similiarly as above, indicating that the parent virtual slot contains a specific `Bindable` event name.

To support `[Bindable]`, the bytecode generator generates event dispatch code right after the property's assignment, whether within a destructuring assignment that affects the enclosing class's instance or a direct assignment.

## Embed

The `[Embed]` meta-data is used in a few different ways:

* It may appear in a class definition in which case it must extend the right class
* It may appear in a `static` variable definition, consisting of only one binding, possibly `const` (without initializer), typed `Class`.
* It may embed a font with several options, such as Unicode range and `embedAsCFF`
* It may embed a SWF
* It may embed an octet stream
* It may embed possibly other formats not listed here.

## Event

The `[Event]` meta-data is used to indicate a dispatched event in a class or interface.