# Verifier

## External definitions

In external definitions (`[whack_external(...)]` or these contained within a class that has this meta-data), the verifier puts some restrictions, such as requiring only `native` or `abstract` methods, empty package and global initialization code, and variable bindings may only be assigned a constant.

## Meta-data

* [ ] Handle Whack `[Bindable]`
* [ ] Handle Whack `[Embed]`

### @copy

* [ ] Correct anchor links from original source path to substitution source path.

### @inheritDoc

* [ ] Correct anchor links from original source path to substitution source path.

## Attributes

* [ ] Restrict definitions at package block to be either `public` or `internal`.
* [ ] Restrict definitions at top-level to be `internal`.
* [ ] Definitions at the top-level of a class may be in any namespace.
* [ ] Restrict user-defined namespaces to be used only at the top-level of class definitions.

## Options classes

* [ ] Restrain all fields to be writable.

# Virtual slots

* [ ] Delegate `[Bindable]` meta-data's semantic from setter or getter to the virtual slot they belong to.