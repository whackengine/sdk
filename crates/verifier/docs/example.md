# Example

```rust
use whackengine_verifier::ns::*;

// The ActionScript 3 semantic database
let db = Database::new(Default::default());

let verifier = Verifier::new(&db);

// Base compiler options for the verifier
// (note that compilation units have distinct compiler options
// that must be set manually)
let compiler_options = Rc::new(CompilerOptions::default());

// List of ActionScript 3 programs
let as3_programs: Vec<Rc<Program>> = vec![];

// List of MXML sources (they are not taken into consideration for now)
let mxml_list: Vec<Rc<Mxml>> = vec![];

// Verify programs
verifier.verify_programs(&compiler_options, as3_programs, mxml_list);

// Unused(&db).all().borrow().iter() = yields unused (nominal and located) entities
// which you can report a warning over.

// Each compilation unit will now have diagnostics.
let example_diagnostics = as3_programs[i].location.compilation_unit().nested_diagnostics(); 

if !verifier.invalidated() {
    // Database::node_mapping() yields a mapping (a "NodeAssignment" object)
    // from a node to an "Entity", where the node is one that is behind a "Rc" pointer.
    let entity = db.node_mapping().get(&any_node); // Option<Entity>
}
```

Examples of node mapping:

* `Rc<Program>` is mapped to an `Activation` entity used by top level directives (not packages themselves). `Activation` is a `Scope`; and in case of `Program`s they do declare "public" and "internal" namespaces.
* Blocks in general are mapped to a `Scope` entity.