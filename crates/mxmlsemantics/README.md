# whackengine::mxmlsemantics

whackengine::mxmlsemantics is a Rust library for creating, inspecting and modifying the semantic data of the ActionScript 3 language ahead of time.

whackengine::mxmlsemantics implements three dimensional names, property lookup, conversion, number representation, interface implementation log, method overriding, applying parameterized types, environment variable cache, unused entity tracking, a factory, and several entities (for example, classes, methods and variables).

whackengine::mxmlsemantics does not include anything related to the Adobe Flex framework; these are implemented through a compiler.

## Example

Create a package `foo.bar` and log its fully qualified name:

```rust
let db = Database::new(Default::default());
let foo_bar = db.factory().create_package(["foo", "bar"]);
println!("Package name: {}", foo_bar.fully_qualified_name());
```

## Global object requisites

The minimum requisites for the framework globals so that the `whackengine::mxmlsemantics` database does not emit an infinite `DeferError` includes defining the following classes. Ensure you have defined them; the properties and methods are not required within them in `whackengine::mxmlsemantics`.

- Object
- Boolean
- Number
- int
- uint
- float
- String
- Array
- Namespace
- Function
- Class
- XML
- XMLList
- RegExp
- Date
- Promise.\<T\>
- Vector.\<T\> (in the top-level package)
- Map.\<K, V>
- framework.util.ByteArray
- framework.util.Proxy

## License

Apache 2.0