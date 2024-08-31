use crate::ns::*;

pub(crate) struct DirectiveSubverifier;

impl DirectiveSubverifier {
    pub fn verify_directives(verifier: &mut Subverifier, list: &[Rc<Directive>]) -> Result<(), DeferError> {
        let mut any_defer = false;
        for drtv in list {
            let r = Self::verify_directive(verifier, drtv).is_err();
            any_defer = any_defer || r;
        }
        if any_defer { Err(DeferError(None)) } else { Ok(()) }
    }

    pub fn verify_directive(verifier: &mut Subverifier, drtv: &Rc<Directive>) -> Result<(), DeferError> {
        match drtv.as_ref() {
            Directive::VariableDefinition(defn) => {
                Self::verify_var_defn(verifier, drtv, defn)
            },
            Directive::FunctionDefinition(defn) => {
                Self::verify_fn_defn(verifier, drtv, defn)
            },
            Directive::ClassDefinition(defn) => {
                Self::verify_class_defn(verifier, drtv, defn)
            },
            Directive::Block(block) => {
                let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
                if phase == VerifierPhase::Finished {
                    return Ok(());
                }
                let host = verifier.host.clone();
                let scope = host.lazy_node_mapping(drtv, || {
                    host.factory().create_scope()
                });
                verifier.inherit_and_enter_scope(&scope);
                let any_defer = Self::verify_directives(verifier, &block.directives).is_err();
                verifier.exit_scope();
                if any_defer {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            Directive::LabeledStatement(lstmt) => {
                Self::verify_directive(verifier, &lstmt.substatement)
            },
            Directive::IfStatement(ifstmt) => {
                let mut any_defer = Self::verify_directive(verifier, &ifstmt.consequent).is_err();
                if let Some(alt) = &ifstmt.alternative {
                    let r = Self::verify_directive(verifier, alt).is_err();
                    any_defer = any_defer || r;
                }
                if any_defer { Err(DeferError(None)) } else { Ok(()) }
            },
            Directive::SwitchStatement(swstmt) => {
                let mut any_defer = false;
                for case in &swstmt.cases {
                    let r = Self::verify_directives(verifier, &case.directives).is_err();
                    any_defer = any_defer || r;
                }
                if any_defer { Err(DeferError(None)) } else { Ok(()) }
            },
            Directive::SwitchTypeStatement(swstmt) => {
                let mut any_defer = false;
                for case in &swstmt.cases {
                    let r = Self::verify_block(verifier, &case.block).is_err();
                    any_defer = any_defer || r;
                }
                if any_defer { Err(DeferError(None)) } else { Ok(()) }
            },
            Directive::DoStatement(dostmt) => {
                Self::verify_directive(verifier, &dostmt.body)
            },
            Directive::WhileStatement(whilestmt) => {
                Self::verify_directive(verifier, &whilestmt.body)
            },
            Directive::ForStatement(forstmt) => {
                let scope = verifier.host.lazy_node_mapping(drtv, || {
                    verifier.host.factory().create_scope()
                });
                verifier.inherit_and_enter_scope(&scope);
                let r = Self::verify_directive(verifier, &forstmt.body);
                verifier.exit_scope();
                r
            },
            Directive::ForInStatement(forstmt) => {
                let scope = verifier.host.lazy_node_mapping(drtv, || {
                    verifier.host.factory().create_scope()
                });
                verifier.inherit_and_enter_scope(&scope);
                let r = Self::verify_directive(verifier, &forstmt.body);
                verifier.exit_scope();
                r
            },
            Directive::WithStatement(withstmt) => {
                Self::verify_directive(verifier, &withstmt.body)
            },
            Directive::TryStatement(trystmt) => {
                let mut any_defer = Self::verify_block(verifier, &trystmt.block).is_err();
                for catch_clause in &trystmt.catch_clauses {
                    let r = Self::verify_block(verifier, &catch_clause.block).is_err();
                    any_defer = any_defer || r;
                }
                if let Some(finally_clause) = trystmt.finally_clause.as_ref() {
                    let r = Self::verify_block(verifier, &finally_clause.block).is_err();
                    any_defer = any_defer || r;
                }
                if any_defer { Err(DeferError(None)) } else { Ok(()) }
            },
            Directive::ImportDirective(impdrtv) => {
                Self::verify_import_directive(verifier, drtv, impdrtv)
            },
            Directive::UseNamespaceDirective(usedrtv) => {
                let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
                if phase == VerifierPhase::Finished {
                    return Ok(());
                }
                match phase {
                    VerifierPhase::Alpha => {
                        verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                        Err(DeferError(None))
                    },
                    VerifierPhase::Beta => {
                        Self::verify_use_ns_ns(verifier, &usedrtv.expression)?;
                        verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                        Ok(())
                    },
                    _ => panic!(),
                }
            },
            Directive::IncludeDirective(incdrtv) => {
                if incdrtv.nested_directives.len() == 0 {
                    return Ok(());
                }
                let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
                if phase == VerifierPhase::Finished {
                    return Ok(());
                }
                if Self::verify_directives(verifier, &incdrtv.nested_directives).is_err() {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            Directive::ConfigurationDirective(cfgdrtv) =>
                Self::verify_config_drtv(verifier, drtv, cfgdrtv),
            Directive::PackageConcatDirective(pckgcat) =>
                Self::verify_package_concat_drtv(verifier, drtv, pckgcat),
            Directive::DirectiveInjection(inj) => {
                let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
                if phase == VerifierPhase::Finished {
                    return Ok(());
                }
                if Self::verify_directives(verifier, inj.directives.borrow().as_ref()).is_err() {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    fn verify_class_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &ClassDefinition) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            // Alpha
            VerifierPhase::Alpha => {
                // Determine the class's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_never_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (class_parent_scope, class_parent, mut class_out, ns) = defn_local.unwrap();

                let public_ns = class_parent_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Public).unwrap();
                let name = verifier.host.factory().create_qname(&ns, defn.name.0.clone());
                let mut class_entity = verifier.host.factory().create_class_type(name, &public_ns);
                class_entity.set_parent(Some(class_parent.clone()));
                class_entity.set_asdoc(defn.asdoc.clone());
                class_entity.set_location(Some(defn.name.1.clone()));
                let metadata = Attribute::find_metadata(&defn.attributes);
                for m in metadata {
                    // [Options] meta-data
                    if m.name.0 == "Options" {
                        class_entity.set_is_options_class(true);
                    // [Whack::External] meta-data
                    } else if m.name.0 == "Whack::External" {
                        let mut slots = 0usize;

                        // Require the `slots="NUMBER"` pair,
                        // defining the number of elements contained in the instance Array
                        // at runtime (always counts the CONSTRUCTOR and DYNAMIC
                        // PROPERTIES slots, therefore it is at least "2").
                        if let Some(entries) = m.entries.as_ref() {
                            let mut found_slots = false;
                            for entry in entries {
                                if let Some(k) = entry.key {
                                    if k.0 == "slots" {
                                        use std::str::FromStr;
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(v) => v.0.clone(),
                                            MetadataValue::IdentifierString(v) => v.0.clone(),
                                        };
                                        slots = usize::from_str(&val).unwrap_or(0);
                                        found_slots = true;
                                    }
                                }
                            }
                            if !found_slots {
                                verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::ExternalClassMustSetSlots, diagarg![]);
                            }
                        } else {
                            verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::ExternalClassMustSetSlots, diagarg![]);
                        }

                        // Set slots number
                        verifier.codegen_class_info(&class_entity).slots.set(slots);

                        // Mark as external
                        class_entity.set_is_external(true);
                    }
                }
                class_entity.metadata().extend(metadata);
                class_entity.set_is_static(Attribute::find_static(&defn.attributes).is_some());
                class_entity.set_is_dynamic(Attribute::find_dynamic(&defn.attributes).is_some());
                class_entity.set_is_abstract(Attribute::find_abstract(&defn.attributes).is_some());
                class_entity.set_is_final(Attribute::find_final(&defn.attributes).is_some());
                class_entity.set_extends_class(Some(verifier.host.unresolved_entity()));

                // Attempt to define the class partially;
                // or fail if a conflict occurs, therefore ignoring
                // this class definition.
                if let Some(prev) = class_out.get(&name) {
                    class_entity = verifier.handle_definition_conflict(&prev, &class_entity);
                } else {
                    Unused(&verifier.host).add_nominal(&class_entity);
                    class_out.set(name, class_entity.clone());
                }
                if !class_entity.is::<ClassType>() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                fixme()
            },
            _ => panic!(),
        }
    }

    fn verify_var_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &VariableDefinition) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        // Determine the variable's scope, parent, property destination, and namespace.
        let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
        if defn_local.is_err() {
            verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
            return Ok(());
        }
        let (var_scope, var_parent, mut var_out, ns) = defn_local.unwrap();

        // Determine whether the definition is external or not
        let is_external = if var_parent.is::<Type>() && var_parent.is_external() {
            true
        } else {
            // [Whack::External]
            defn.attributes.iter().find(|a| {
                if let Attribute::Metadata(m) = a { m.name.0 == "Whack::External" } else { false }
            }).is_some()
        };

        match phase {
            // Alpha
            VerifierPhase::Alpha => {
                for binding in &defn.bindings {
                    let is_destructuring = !(matches!(binding.destructuring.destructuring.as_ref(), Expression::QualifiedIdentifier(_)));

                    // If the parent is a fixture or if the variable is external,
                    // do not allow destructuring, in which case the pattern shall be invalidated.
                    if is_destructuring && (var_scope.is::<FixtureScope>() || is_external) {
                        verifier.add_verify_error(&binding.destructuring.location, WhackDiagnosticKind::CannotUseDestructuringHere, diagarg![]);
                        verifier.host.node_mapping().set(&binding.destructuring.destructuring, Some(verifier.host.invalidation_entity()));
                        continue;
                    }

                    // Verify identifier binding or destructuring pattern (alpha)
                    let _ = DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &verifier.host.unresolved_entity(), defn.kind.0 == VariableDefinitionKind::Const, &mut var_out, &ns, &var_parent, is_external);
                }

                // Set ASDoc and meta-data
                let slot1 = verifier.host.node_mapping().get(&defn.bindings[0].destructuring.destructuring);
                if slot1.as_ref().and_then(|e| if e.is::<VariableSlot>() { Some(e) } else { None }).is_some() {
                    let slot1 = slot1.unwrap();
                    slot1.set_asdoc(defn.asdoc.clone());
                    slot1.metadata().extend(Attribute::find_metadata(&defn.attributes));
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            // Beta
            VerifierPhase::Beta => {
                for binding in &defn.bindings {
                    // If a binding is a simple identifier,
                    // try resolving type annotation if any; if resolved,
                    // if the binding's slot is not invalidated
                    // update the binding slot's static type.
                    let is_simple_id = matches!(binding.destructuring.destructuring.as_ref(), Expression::QualifiedIdentifier(_));
                    if is_simple_id && binding.destructuring.type_annotation.is_some() {
                        let t = verifier.verify_type_expression(binding.destructuring.type_annotation.as_ref().unwrap())?;
                        if let Some(t) = t {
                            let slot = verifier.node_mapping().get(&binding.destructuring.destructuring);
                            if let Some(slot) = slot {
                                if slot.is::<VariableSlot>() {
                                    slot.set_static_type(t);
                                }
                            }
                        }
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                Err(DeferError(None))
            },
            // Delta
            VerifierPhase::Delta => {
                for binding in &defn.bindings {
                    // If a binding is a simple identifier and
                    // the binding's slot is not invalidated and its static type is unresolved,
                    // try resolving the type annotation if any; if resolved,
                    // update the binding slot's static type.
                    let is_simple_id = matches!(binding.destructuring.destructuring.as_ref(), Expression::QualifiedIdentifier(_));
                    if is_simple_id {
                        let slot = verifier.node_mapping().get(&binding.destructuring.destructuring);
                        if let Some(slot) = slot {
                            if slot.is::<VariableSlot>() && slot.static_type(&verifier.host).is::<UnresolvedEntity>() {
                                if binding.destructuring.type_annotation.is_some() {
                                    let t = verifier.verify_type_expression(binding.destructuring.type_annotation.as_ref().unwrap())?;
                                    if let Some(t) = t {
                                        slot.set_static_type(t);
                                    }
                                }
                            }
                        }
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Epsilon);
                Err(DeferError(None))
            },
            // Epsilon
            VerifierPhase::Epsilon => {
                // @todo
                // - Handle the `[Bindable]` meta-data for simple identifier patterns
                // - Handle the `[Embed]` meta-data for simple identifier patterns

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            // Omega
            VerifierPhase::Omega => {
                let is_const = defn.kind.0 == VariableDefinitionKind::Const;

                for i in 0..defn.bindings.len() {
                    let binding = &defn.bindings[i];

                    // Let *init* be `None`.
                    let mut init: Option<Entity> = None;

                    // Try resolving type annotation if any.
                    let mut annotation_type: Option<Entity> = None;
                    if let Some(node) = binding.destructuring.type_annotation.as_ref() {
                        annotation_type = verifier.verify_type_expression(node)?;
                    }

                    // If there is an initialiser and there is a type annotation,
                    // then implicitly coerce it to the annotated type and assign the result to *init*;
                    // otherwise, assign the result of verifying the initialiser into *init*.
                    if let Some(init_node) = binding.initializer.as_ref() {
                        if let Some(t) = annotation_type.as_ref() {
                            init = verifier.imp_coerce_exp(init_node, t)?;
                        } else {
                            init = verifier.verify_expression(init_node, &Default::default())?;
                        }
                    }

                    let host = verifier.host.clone();

                    // Lazy initialise *init1* (`cached_var_init`)
                    let init = verifier.cache_var_init(&binding.destructuring.destructuring, || {
                        // If "init" is Some, return it.
                        if let Some(init) = init {
                            init
                        } else {
                            // If there is a type annotation, then return a value of that type;
                            // otherwise return a value of the `*` type.
                            if let Some(t) = annotation_type {
                                host.factory().create_value(&t)
                            } else {
                                host.factory().create_value(&host.any_type())
                            }
                        }
                    });

                    // If the variable is external, *init* must be a compile-time constant.
                    if is_external {
                        if !init.is::<Constant>() && binding.initializer.is_some() {
                            verifier.add_verify_error(&binding.initializer.as_ref().unwrap().location(), WhackDiagnosticKind::EntityIsNotAConstant, diagarg![]);
                        }
                    }

                    // Verify the identifier binding or destructuring pattern
                    DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &init, is_const, &mut var_out, &ns, &var_parent, is_external)?;

                    // Remove *init1* from "cached_var_init"
                    verifier.cached_var_init.remove(&ByAddress(binding.destructuring.destructuring.clone()));

                    // If there is no type annotation and initialiser is unspecified,
                    // then report a warning
                    if binding.destructuring.type_annotation.is_none() && binding.initializer.is_none() {
                        verifier.add_warning(&binding.destructuring.location, WhackDiagnosticKind::VariableHasNoTypeAnnotation, diagarg![]);
                    }

                    // If variable is marked constant, is not `[Embed]` and does not contain an initializer,
                    // then report an error
                    if is_const && !(i == 0 && Attribute::find_metadata(&defn.attributes).iter().any(|mdata| mdata.name.0 == "Embed")) {
                        verifier.add_verify_error(&binding.destructuring.location, WhackDiagnosticKind::ConstantMustContainInitializer, diagarg![]);
                    }
                }

                // Finish
                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    /// Returns (var_scope, var_parent, var_out, ns) for a
    /// annotatable driective.
    fn definition_local_maybe_static(verifier: &mut Subverifier, attributes: &[Attribute]) -> Result<Result<(Entity, Entity, Names, Entity), ()>, DeferError> {
        // Check the "static" attribute to know where the output name goes in exactly.
        let is_static = Attribute::find_static(&attributes).is_some();
        let mut var_scope = verifier.scope();
        var_scope = if is_static { var_scope.search_hoist_scope() } else { var_scope };
        let var_parent = if var_scope.is::<ClassScope>() || var_scope.is::<EnumScope>() {
            var_scope.class()
        } else if var_scope.is::<InterfaceScope>() {
            var_scope.interface()
        } else {
            var_scope.clone()
        };
        let var_out = if ((var_parent.is::<ClassType>() || var_parent.is::<EnumType>()) && !is_static) || var_parent.is::<InterfaceType>() {
            var_parent.prototype(&verifier.host)
        } else {
            var_parent.properties(&verifier.host)
        };

        // Determine the namespace according to the attribute combination
        let mut ns = None;
        for attr in attributes.iter().rev() {
            match attr {
                Attribute::Expression(exp) => {
                    let nsconst = verifier.verify_expression(exp, &Default::default())?;
                    if nsconst.as_ref().map(|k| !k.is::<NamespaceConstant>()).unwrap_or(false) {
                        verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::NotANamespaceConstant, diagarg![]);
                        return Ok(Err(()));
                    }
                    if !(var_parent.is::<ClassType>() || var_parent.is::<EnumType>()) {
                        verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::AccessControlNamespaceNotAllowedHere, diagarg![]);
                        return Ok(Err(()));
                    }
                    if nsconst.is_none() {
                        return Ok(Err(()));
                    }
                    ns = Some(nsconst.unwrap().referenced_ns());
                    break;
                },
                Attribute::Public(_) => {
                    ns = var_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Public);
                    break;
                },
                Attribute::Private(loc) => {
                    // protected or static-protected
                    if !var_parent.is::<ClassType>() {
                        verifier.add_verify_error(loc, WhackDiagnosticKind::AccessControlNamespaceNotAllowedHere, diagarg![]);
                        return Ok(Err(()));
                    }
                    ns = var_parent.private_ns();
                    break;
                },
                Attribute::Protected(loc) => {
                    // protected or static-protected
                    if !var_parent.is::<ClassType>() {
                        verifier.add_verify_error(loc, WhackDiagnosticKind::AccessControlNamespaceNotAllowedHere, diagarg![]);
                        return Ok(Err(()));
                    }
                    ns = if is_static { var_parent.static_protected_ns() } else { var_parent.protected_ns() };
                    break;
                },
                Attribute::Internal(_) => {
                    ns = var_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal);
                    break;
                },
                _ => {},
            }
        }
        if ns.is_none() {
            ns = var_scope.search_system_ns_in_scope_chain(if var_parent.is::<InterfaceType>() { SystemNamespaceKind::Public } else { SystemNamespaceKind::Internal });
        }
        let ns = ns.unwrap();

        Ok(Ok((var_scope, var_parent, var_out, ns)))
    }

    /// Returns (var_scope, var_parent, var_out, ns) for a
    /// annotatable directive.
    fn definition_local_never_static(verifier: &mut Subverifier, attributes: &[Attribute]) -> Result<Result<(Entity, Entity, Names, Entity), ()>, DeferError> {
        let var_scope = verifier.scope().search_hoist_scope();
        let var_parent = if var_scope.is::<ClassScope>() || var_scope.is::<EnumScope>() {
            var_scope.class()
        } else if var_scope.is::<InterfaceScope>() {
            var_scope.interface()
        } else {
            var_scope.clone()
        };
        let var_out = var_parent.properties(&verifier.host);

        // Determine the namespace according to the attribute combination
        let mut ns = None;
        for attr in attributes.iter().rev() {
            match attr {
                Attribute::Expression(exp) => {
                    let nsconst = verifier.verify_expression(exp, &Default::default())?;
                    if nsconst.as_ref().map(|k| !k.is::<NamespaceConstant>()).unwrap_or(false) {
                        verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::NotANamespaceConstant, diagarg![]);
                        return Ok(Err(()));
                    }
                    if !(var_parent.is::<ClassType>() || var_parent.is::<EnumType>()) {
                        verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::AccessControlNamespaceNotAllowedHere, diagarg![]);
                        return Ok(Err(()));
                    }
                    if nsconst.is_none() {
                        return Ok(Err(()));
                    }
                    ns = Some(nsconst.unwrap().referenced_ns());
                    break;
                },
                Attribute::Public(_) => {
                    ns = var_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Public);
                    break;
                },
                Attribute::Internal(_) => {
                    ns = var_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal);
                    break;
                },
                _ => {},
            }
        }
        if ns.is_none() {
            ns = var_scope.search_system_ns_in_scope_chain(if var_parent.is::<InterfaceType>() { SystemNamespaceKind::Public } else { SystemNamespaceKind::Internal });
        }
        let ns = ns.unwrap();

        Ok(Ok((var_scope, var_parent, var_out, ns)))
    }

    fn verify_fn_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &FunctionDefinition) -> Result<(), DeferError> {
        match &defn.name {
            FunctionName::Identifier(name) => Self::verify_normal_fn_defn(verifier, drtv, defn, name),
            FunctionName::Constructor(name) => Self::verify_constructor_fn_defn(verifier, drtv, defn, name),
            FunctionName::Getter(name) => Self::verify_getter(verifier, drtv, defn, name),
            FunctionName::Setter(name) => Self::verify_setter(verifier, drtv, defn, name),
        }
    }

    fn verify_normal_fn_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &FunctionDefinition, name: &(String, Location)) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                // Determine the property's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, mut fn_out, ns) = defn_local.unwrap();

                // Determine whether the definition is external or not
                let is_external = if fn_parent.is::<Type>() && fn_parent.is_external() {
                    true
                } else {
                    // [Whack::External]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "Whack::External" } else { false }
                    }).is_some()
                };

                // Create method slot
                let common = defn.common.clone();
                let loc = name.1.clone();
                let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                let mut slot = verifier.host.factory().create_method_slot(&name, &verifier.host.unresolved_entity());
                slot.set_location(Some(loc.clone()));
                slot.set_parent(Some(fn_parent.clone()));
                slot.set_is_external(is_external);
                slot.set_is_final(Attribute::find_final(&defn.attributes).is_some());
                slot.set_is_static(Attribute::find_static(&defn.attributes).is_some());
                slot.set_is_native(Attribute::find_native(&defn.attributes).is_some());
                slot.set_is_abstract(Attribute::find_abstract(&defn.attributes).is_some());
                slot.set_is_async(common.contains_await);
                slot.set_is_generator(common.contains_yield);
                slot.set_is_constructor(false);
                slot.set_is_overriding(Attribute::find_override(&defn.attributes).is_some());

                // Set meta-data ASDoc
                slot.metadata().extend(Attribute::find_metadata(&defn.attributes));
                slot.set_asdoc(defn.asdoc.clone());

                // If external, function must be native or abstract.
                if is_external && !(slot.is_native() || slot.is_abstract()) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::ExternalFunctionMustBeNativeOrAbstract, diagarg![]);
                }

                // Define method property
                if let Some(prev) = fn_out.get(&name) {
                    slot = verifier.handle_definition_conflict(&prev, &slot);
                } else {
                    Unused(&verifier.host).add_nominal(&slot);
                    fn_out.set(name, slot.clone());
                }

                // Initialise activation
                if slot.is::<MethodSlot>() {
                    let act = verifier.host.factory().create_activation(&slot);
                    slot.set_activation(Some(act.clone()));
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map node to method slot
                verifier.host.node_mapping().set(drtv, if slot.is::<MethodSlot>() { Some(slot.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Database
                let host = verifier.host.clone();

                // Determine definition location
                let loc = name.1.clone();
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, fn_out, ns) = defn_local.unwrap();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials (1)
                let mut partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone()));
                if partials.is_none() {
                    // The "this" receiver
                    if let Some(this_param) = common.signature.this_parameter.clone() {
                        let t = verifier.verify_type_expression(&this_param.type_annotation)?.unwrap_or(host.any_type());
                        activation.set_this(Some(host.factory().create_this_object(&t)));
                    } else if !slot.is_static() && (fn_parent.is::<ClassType>() || fn_parent.is::<EnumType>()) {
                        activation.set_this(Some(host.factory().create_this_object(&fn_parent)));
                    } else {
                        // Inherit "this" type
                        let super_act = verifier.scope().search_activation();
                        let super_this_type = super_act.and_then(|a| a.this().map(|this| this.static_type(&verifier.host)));
                        activation.set_this(Some(host.factory().create_this_object(&super_this_type.unwrap_or(host.any_type()))));
                    }

                    let partials1 = VerifierFunctionPartials::new(&activation, &loc);
                    verifier.function_definition_partials.set(NodeAsKey(common.clone()), partials1.clone());
                    partials = Some(partials1);
                }

                // Definition partials (2)
                let partials = partials.unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                // Verify parameter bindings
                let mut params: Vec<Rc<SemanticFunctionTypeParameter>> = vec![];
                let mut last_param_kind = ParameterKind::Required;        
                if partials.params().is_none() {
                    let internal_ns = kscope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

                    for param_node in &common.signature.parameters {
                        match param_node.kind {
                            ParameterKind::Required => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Optional => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init;
                                if let Some(init1) = verifier.cached_var_init.get(&NodeAsKey(pattern.clone())) {
                                    init = init1.clone();
                                } else {
                                    init = verifier.imp_coerce_exp(param_node.default_value.as_ref().unwrap(), &param_type)?.unwrap_or(host.invalidation_entity());
                                    verifier.cached_var_init.insert(NodeAsKey(pattern.clone()), init.clone());
                                    if !init.is::<InvalidationEntity>() && !init.static_type(&host).is::<Constant>() {
                                        verifier.add_verify_error(&param_node.default_value.as_ref().unwrap().location(), WhackDiagnosticKind::EntityIsNotAConstant, diagarg![]);
                                    }
                                }
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, &param_node.destructuring.destructuring, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Rest => {
                                let mut param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]));
                                    if param_type.array_element_type(&host)?.is_none() {
                                        verifier.add_verify_error(&type_annot.location(), WhackDiagnosticKind::RestParameterMustBeArray, diagarg![]);
                                        param_type = host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                                    }
                                } else {
                                    param_type = host.array_type_of_any()?;
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) && last_param_kind != ParameterKind::Rest {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                        }
                        last_param_kind = param_node.kind;
                    }
        
                    partials.set_params(Some(params));
                }
        
                // Result type
                if let Some(result_annot) = common.signature.result_type.as_ref() {
                    if partials.result_type().is_none() {
                        let result_type = verifier.verify_type_expression(result_annot)?.unwrap_or(host.invalidation_entity());
                        partials.set_result_type(Some(result_type));
                    }
                } else if partials.result_type().is_none() {
                    verifier.add_warning(&loc, WhackDiagnosticKind::ReturnValueHasNoTypeDeclaration, diagarg![]);
                    partials.set_result_type(Some(if common.contains_await { host.promise_type_of_any()? } else { host.any_type() }));
                }

                // Set signature
                let signature;
                if partials.signature().is_none() {
                    let mut result_type = partials.result_type().unwrap(); 

                    if common.contains_await && !result_type.promise_result_type(&host)?.is_some() {
                        verifier.add_verify_error(&loc, WhackDiagnosticKind::ReturnTypeDeclarationMustBePromise, diagarg![]);
                        result_type = host.promise_type().defer()?.apply_type(&host, &host.promise_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()])
                    }

                    let signature1 = host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type);
                    partials.set_signature(Some(signature1.clone()));
                    signature = signature1;
                } else {
                    signature = partials.signature().unwrap();
                }
                slot.set_signature(&signature);

                // "override"
                let marked_override = Attribute::find_override(&defn.attributes).is_some();

                // Do not allow shadowing properties in base classes if not marked "override".
                if !marked_override {
                    let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                    verifier.ensure_not_shadowing_definition(&loc, &fn_out, &fn_parent, &name);
                }

                // Restore scope
                verifier.set_scope(&kscope);

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                Err(DeferError(None))
            },
            VerifierPhase::Delta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Database
                let host = verifier.host.clone();

                // Definition location
                let loc = name.1.clone();

                // Override if marked "override"
                if slot.is_overriding() {
                    match MethodOverride(&host).override_method(&slot, &verifier.scope().concat_open_ns_set_of_scope_chain()) {
                        Ok(_) => {},
                        Err(MethodOverrideError::Defer) => {
                            return Err(DeferError(None));
                        },
                        Err(MethodOverrideError::IncompatibleOverride { expected_signature, actual_signature }) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::IncompatibleOverride, diagarg![expected_signature.clone(), actual_signature.clone()]);
                        },
                        Err(MethodOverrideError::MustOverrideAMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::MustOverrideAMethod, diagarg![]);
                        },
                        Err(MethodOverrideError::OverridingFinalMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::OverridingFinalMethod, diagarg![]);
                        },
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials
                let partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone())).unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                FunctionCommonSubverifier::verify_function_definition_common(verifier, &common, &partials)?;

                // Restore scope
                verifier.set_scope(&kscope);

                // Finish
                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_constructor_fn_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &FunctionDefinition, name: &(String, Location)) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                let fn_scope = verifier.scope().search_hoist_scope();
                let fn_parent = fn_scope.class();

                // Determine whether the definition is external or not
                let is_external = fn_parent.is_external();

                // Create method slot
                let loc = name.1.clone();
                let ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Public).unwrap();
                let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                let mut slot = verifier.host.factory().create_method_slot(&name, &verifier.host.unresolved_entity());
                slot.set_location(Some(loc.clone()));
                slot.set_parent(Some(fn_parent.clone()));
                slot.set_is_external(is_external);
                slot.set_is_native(Attribute::find_native(&defn.attributes).is_some());
                slot.set_is_constructor(false);

                // Set meta-data ASDoc
                slot.metadata().extend(Attribute::find_metadata(&defn.attributes));
                slot.set_asdoc(defn.asdoc.clone());

                // If external, function must be native.
                if is_external && !slot.is_native() {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::ExternalFunctionMustBeNativeOrAbstract, diagarg![]);
                }

                // Define constructor
                if fn_parent.constructor_method(&verifier.host).is_some() {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::RedefiningConstructor, diagarg![]);
                    slot = verifier.host.invalidation_entity();
                } else {
                    fn_parent.set_constructor_method(Some(slot.clone()));
                }

                // Initialise activation
                if slot.is::<MethodSlot>() {
                    let act = verifier.host.factory().create_activation(&slot);
                    slot.set_activation(Some(act.clone()));
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map node to method slot
                verifier.host.node_mapping().set(drtv, if slot.is::<MethodSlot>() { Some(slot.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Database
                let host = verifier.host.clone();

                // Determine definition location
                let loc = name.1.clone();
                let fn_scope = verifier.scope().search_hoist_scope();
                let fn_parent = fn_scope.class();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials (1)
                let mut partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone()));
                if partials.is_none() {
                    // The "this" receiver
                    if let Some(this_param) = common.signature.this_parameter.clone() {
                        let t = verifier.verify_type_expression(&this_param.type_annotation)?.unwrap_or(host.any_type());
                        activation.set_this(Some(host.factory().create_this_object(&t)));
                    } else if !slot.is_static() && (fn_parent.is::<ClassType>() || fn_parent.is::<EnumType>()) {
                        activation.set_this(Some(host.factory().create_this_object(&fn_parent)));
                    } else {
                        // Inherit "this" type
                        let super_act = verifier.scope().search_activation();
                        let super_this_type = super_act.and_then(|a| a.this().map(|this| this.static_type(&verifier.host)));
                        activation.set_this(Some(host.factory().create_this_object(&super_this_type.unwrap_or(host.any_type()))));
                    }

                    let partials1 = VerifierFunctionPartials::new(&activation, &loc);
                    verifier.function_definition_partials.set(NodeAsKey(common.clone()), partials1.clone());
                    partials = Some(partials1);
                }

                // Definition partials (2)
                let partials = partials.unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                // Verify parameter bindings
                let mut params: Vec<Rc<SemanticFunctionTypeParameter>> = vec![];
                let mut last_param_kind = ParameterKind::Required;        
                if partials.params().is_none() {
                    let internal_ns = kscope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

                    for param_node in &common.signature.parameters {
                        match param_node.kind {
                            ParameterKind::Required => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Optional => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init;
                                if let Some(init1) = verifier.cached_var_init.get(&NodeAsKey(pattern.clone())) {
                                    init = init1.clone();
                                } else {
                                    init = verifier.imp_coerce_exp(param_node.default_value.as_ref().unwrap(), &param_type)?.unwrap_or(host.invalidation_entity());
                                    verifier.cached_var_init.insert(NodeAsKey(pattern.clone()), init.clone());
                                    if !init.is::<InvalidationEntity>() && !init.static_type(&host).is::<Constant>() {
                                        verifier.add_verify_error(&param_node.default_value.as_ref().unwrap().location(), WhackDiagnosticKind::EntityIsNotAConstant, diagarg![]);
                                    }
                                }
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, &param_node.destructuring.destructuring, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Rest => {
                                let mut param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]));
                                    if param_type.array_element_type(&host)?.is_none() {
                                        verifier.add_verify_error(&type_annot.location(), WhackDiagnosticKind::RestParameterMustBeArray, diagarg![]);
                                        param_type = host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                                    }
                                } else {
                                    param_type = host.array_type_of_any()?;
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) && last_param_kind != ParameterKind::Rest {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                        }
                        last_param_kind = param_node.kind;
                    }
        
                    partials.set_params(Some(params));
                }
        
                // Result type (1)
                if let Some(result_annot) = common.signature.result_type.as_ref() {
                    if partials.result_type().is_none() {
                        let _ = verifier.verify_type_expression(result_annot)?;
                    }
                }

                // Result type (2)
                if partials.result_type().is_none() {
                    partials.set_result_type(Some(host.void_type()));
                }

                // Set signature
                let signature;
                if partials.signature().is_none() {
                    let result_type = partials.result_type().unwrap(); 
                    let signature1 = host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type);
                    partials.set_signature(Some(signature1.clone()));
                    signature = signature1;
                } else {
                    signature = partials.signature().unwrap();
                }
                slot.set_signature(&signature);

                // Restore scope
                verifier.set_scope(&kscope);

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                Err(DeferError(None))
            },
            VerifierPhase::Delta => {
                // FunctionCommon
                let common = defn.common.clone();

                // Determine definition location
                let loc = name.1.clone();
                let fn_scope = verifier.scope().search_hoist_scope();
                let fn_parent = fn_scope.class();

                let base_class = fn_parent.extends_class(&verifier.host);
                if let Some(base_class) = base_class {
                    if let Some(ctor_m) = base_class.constructor_method(&verifier.host) {
                        let sig = ctor_m.signature(&verifier.host).defer()?;
                        if sig.params().iter().any(|p| p.kind == ParameterKind::Required) {
                            let super_found = match common.body.as_ref() {
                                Some(FunctionBody::Block(block)) =>
                                    block.directives.iter().any(|d| matches!(d.as_ref(), Directive::SuperStatement(_))),
                                Some(FunctionBody::Expression(_)) => false,
                                None => true,
                            };
                            if !super_found {
                                verifier.add_verify_error(&loc, WhackDiagnosticKind::ConstructorMustContainSuperStatement, diagarg![]);
                            }
                        }
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials
                let partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone())).unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                FunctionCommonSubverifier::verify_function_definition_common(verifier, &common, &partials)?;

                // Restore scope
                verifier.set_scope(&kscope);

                // Finish
                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_getter(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &FunctionDefinition, name: &(String, Location)) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                // Determine the property's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, mut fn_out, ns) = defn_local.unwrap();

                // Determine whether the definition is external or not
                let is_external = if fn_parent.is::<Type>() && fn_parent.is_external() {
                    true
                } else {
                    // [Whack::External]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "Whack::External" } else { false }
                    }).is_some()
                };

                // Create method slot
                let loc = name.1.clone();
                let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                let mut slot = verifier.host.factory().create_method_slot(&name, &verifier.host.unresolved_entity());
                slot.set_location(Some(loc.clone()));
                slot.set_parent(Some(fn_parent.clone()));
                slot.set_is_external(is_external);
                slot.set_is_final(Attribute::find_final(&defn.attributes).is_some());
                slot.set_is_static(Attribute::find_static(&defn.attributes).is_some());
                slot.set_is_native(Attribute::find_native(&defn.attributes).is_some());
                slot.set_is_abstract(Attribute::find_abstract(&defn.attributes).is_some());
                slot.set_is_constructor(false);
                slot.set_is_overriding(Attribute::find_override(&defn.attributes).is_some());

                // If external, function must be native or abstract.
                if is_external && !(slot.is_native() || slot.is_abstract()) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::ExternalFunctionMustBeNativeOrAbstract, diagarg![]);
                }

                // Define function
                let mut virtual_var: Option<Entity> = None;
                if let Some(prev) = fn_out.get(&name) {
                    if prev.is::<VirtualSlot>() && prev.getter(&verifier.host).is_none() {
                        virtual_var = Some(prev.clone());
                    } else {
                        slot = verifier.handle_definition_conflict(&prev, &slot);
                    }
                } else {
                    let virtual_var1 = verifier.host.factory().create_virtual_slot(&name);
                    virtual_var1.set_is_external(is_external);
                    virtual_var = Some(virtual_var1.clone());
                    Unused(&verifier.host).add_nominal(&virtual_var1);
                    fn_out.set(name, virtual_var1.clone());
                }

                if let Some(virtual_var) = virtual_var {
                    // Function attachment
                    virtual_var.set_getter(Some(slot.clone()));
                    slot.set_of_virtual_slot(Some(virtual_var.clone()));

                    // Set meta-data ASDoc
                    virtual_var.metadata().extend(Attribute::find_metadata(&defn.attributes));
                    virtual_var.set_asdoc(virtual_var.asdoc().or(defn.asdoc.clone()));

                    // Set location
                    virtual_var.set_location(virtual_var.location().or(slot.location()));
                }

                // Initialise activation
                if slot.is::<MethodSlot>() {
                    let act = verifier.host.factory().create_activation(&slot);
                    slot.set_activation(Some(act.clone()));
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map node to method slot
                verifier.host.node_mapping().set(drtv, if slot.is::<MethodSlot>() { Some(slot.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Database
                let host = verifier.host.clone();

                // Determine definition location
                let loc = name.1.clone();
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, fn_out, ns) = defn_local.unwrap();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials (1)
                let mut partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone()));
                if partials.is_none() {
                    // The "this" receiver
                    if let Some(this_param) = common.signature.this_parameter.clone() {
                        let t = verifier.verify_type_expression(&this_param.type_annotation)?.unwrap_or(host.any_type());
                        activation.set_this(Some(host.factory().create_this_object(&t)));
                    } else if !slot.is_static() && (fn_parent.is::<ClassType>() || fn_parent.is::<EnumType>()) {
                        activation.set_this(Some(host.factory().create_this_object(&fn_parent)));
                    } else {
                        // Inherit "this" type
                        let super_act = verifier.scope().search_activation();
                        let super_this_type = super_act.and_then(|a| a.this().map(|this| this.static_type(&verifier.host)));
                        activation.set_this(Some(host.factory().create_this_object(&super_this_type.unwrap_or(host.any_type()))));
                    }

                    let partials1 = VerifierFunctionPartials::new(&activation, &loc);
                    verifier.function_definition_partials.set(NodeAsKey(common.clone()), partials1.clone());
                    partials = Some(partials1);
                }

                // Definition partials (2)
                let partials = partials.unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                // Verify parameter bindings
                let mut params: Vec<Rc<SemanticFunctionTypeParameter>> = vec![];
                let mut last_param_kind = ParameterKind::Required;        
                if partials.params().is_none() {
                    let internal_ns = kscope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

                    for param_node in &common.signature.parameters {
                        match param_node.kind {
                            ParameterKind::Required => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Optional => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init;
                                if let Some(init1) = verifier.cached_var_init.get(&NodeAsKey(pattern.clone())) {
                                    init = init1.clone();
                                } else {
                                    init = verifier.imp_coerce_exp(param_node.default_value.as_ref().unwrap(), &param_type)?.unwrap_or(host.invalidation_entity());
                                    verifier.cached_var_init.insert(NodeAsKey(pattern.clone()), init.clone());
                                    if !init.is::<InvalidationEntity>() && !init.static_type(&host).is::<Constant>() {
                                        verifier.add_verify_error(&param_node.default_value.as_ref().unwrap().location(), WhackDiagnosticKind::EntityIsNotAConstant, diagarg![]);
                                    }
                                }
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, &param_node.destructuring.destructuring, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Rest => {
                                let mut param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]));
                                    if param_type.array_element_type(&host)?.is_none() {
                                        verifier.add_verify_error(&type_annot.location(), WhackDiagnosticKind::RestParameterMustBeArray, diagarg![]);
                                        param_type = host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                                    }
                                } else {
                                    param_type = host.array_type_of_any()?;
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) && last_param_kind != ParameterKind::Rest {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                        }
                        last_param_kind = param_node.kind;
                    }

                    if params.len() != 0 {
                        verifier.add_verify_error(&loc, WhackDiagnosticKind::GetterMustTakeNoParameters, diagarg![]);
                        params.clear();
                    }
        
                    partials.set_params(Some(params));
                }
        
                // Result type
                if let Some(result_annot) = common.signature.result_type.as_ref() {
                    if partials.result_type().is_none() {
                        let result_type = verifier.verify_type_expression(result_annot)?.unwrap_or(host.invalidation_entity());
                        partials.set_result_type(Some(result_type));
                    }
                } else if partials.result_type().is_none() {
                    verifier.add_warning(&loc, WhackDiagnosticKind::ReturnValueHasNoTypeDeclaration, diagarg![]);
                    partials.set_result_type(Some(host.any_type()));
                }

                // Set signature
                let signature;
                if partials.signature().is_none() {
                    let result_type = partials.result_type().unwrap();
                    let signature1 = host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type);
                    partials.set_signature(Some(signature1.clone()));
                    signature = signature1;
                } else {
                    signature = partials.signature().unwrap();
                }
                slot.set_signature(&signature);

                // "override"
                let marked_override = Attribute::find_override(&defn.attributes).is_some();

                // Do not allow shadowing properties in base classes if not marked "override".
                if !marked_override {
                    let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                    verifier.ensure_not_shadowing_definition(&loc, &fn_out, &fn_parent, &name);
                }

                // Restore scope
                verifier.set_scope(&kscope);

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                Err(DeferError(None))
            },
            VerifierPhase::Delta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Database
                let host = verifier.host.clone();

                // Definition location
                let loc = name.1.clone();

                // Virtual slot
                let virtual_var = slot.of_virtual_slot(&verifier.host).unwrap();
                
                // Ensure the getter returns the correct data type
                if slot.signature(&verifier.host).result_type() != virtual_var.static_type(&verifier.host) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::GetterMustReturnDataType, diagarg![virtual_var.static_type(&verifier.host)]);
                }

                // Override if marked "override"
                if slot.is_overriding() {
                    match MethodOverride(&host).override_method(&slot, &verifier.scope().concat_open_ns_set_of_scope_chain()) {
                        Ok(_) => {},
                        Err(MethodOverrideError::Defer) => {
                            return Err(DeferError(None));
                        },
                        Err(MethodOverrideError::IncompatibleOverride { expected_signature, actual_signature }) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::IncompatibleOverride, diagarg![expected_signature.clone(), actual_signature.clone()]);
                        },
                        Err(MethodOverrideError::MustOverrideAMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::MustOverrideAMethod, diagarg![]);
                        },
                        Err(MethodOverrideError::OverridingFinalMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::OverridingFinalMethod, diagarg![]);
                        },
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials
                let partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone())).unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                FunctionCommonSubverifier::verify_function_definition_common(verifier, &common, &partials)?;

                // Restore scope
                verifier.set_scope(&kscope);

                // Finish
                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_setter(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &FunctionDefinition, name: &(String, Location)) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                // Determine the property's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, mut fn_out, ns) = defn_local.unwrap();

                // Determine whether the definition is external or not
                let is_external = if fn_parent.is::<Type>() && fn_parent.is_external() {
                    true
                } else {
                    // [Whack::External]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "Whack::External" } else { false }
                    }).is_some()
                };

                // Create method slot
                let loc = name.1.clone();
                let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                let mut slot = verifier.host.factory().create_method_slot(&name, &verifier.host.unresolved_entity());
                slot.set_location(Some(loc.clone()));
                slot.set_parent(Some(fn_parent.clone()));
                slot.set_is_external(is_external);
                slot.set_is_final(Attribute::find_final(&defn.attributes).is_some());
                slot.set_is_static(Attribute::find_static(&defn.attributes).is_some());
                slot.set_is_native(Attribute::find_native(&defn.attributes).is_some());
                slot.set_is_abstract(Attribute::find_abstract(&defn.attributes).is_some());
                slot.set_is_constructor(false);
                slot.set_is_overriding(Attribute::find_override(&defn.attributes).is_some());

                // If external, function must be native or abstract.
                if is_external && !(slot.is_native() || slot.is_abstract()) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::ExternalFunctionMustBeNativeOrAbstract, diagarg![]);
                }

                // Define function
                let mut virtual_var: Option<Entity> = None;
                if let Some(prev) = fn_out.get(&name) {
                    if prev.is::<VirtualSlot>() && prev.setter(&verifier.host).is_none() {
                        virtual_var = Some(prev.clone());
                    } else {
                        slot = verifier.handle_definition_conflict(&prev, &slot);
                    }
                } else {
                    let virtual_var1 = verifier.host.factory().create_virtual_slot(&name);
                    virtual_var1.set_is_external(is_external);
                    virtual_var = Some(virtual_var1.clone());
                    Unused(&verifier.host).add_nominal(&virtual_var1);
                    fn_out.set(name, virtual_var1.clone());
                }

                if let Some(virtual_var) = virtual_var {
                    // Function attachment
                    virtual_var.set_setter(Some(slot.clone()));
                    slot.set_of_virtual_slot(Some(virtual_var.clone()));

                    // Set meta-data ASDoc
                    virtual_var.metadata().extend(Attribute::find_metadata(&defn.attributes));
                    virtual_var.set_asdoc(virtual_var.asdoc().or(defn.asdoc.clone()));

                    // Set location
                    virtual_var.set_location(virtual_var.location().or(slot.location()));
                }

                // Initialise activation
                if slot.is::<MethodSlot>() {
                    let act = verifier.host.factory().create_activation(&slot);
                    slot.set_activation(Some(act.clone()));
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map node to method slot
                verifier.host.node_mapping().set(drtv, if slot.is::<MethodSlot>() { Some(slot.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Database
                let host = verifier.host.clone();

                // Determine definition location
                let loc = name.1.clone();
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, fn_parent, fn_out, ns) = defn_local.unwrap();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials (1)
                let mut partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone()));
                if partials.is_none() {
                    // The "this" receiver
                    if let Some(this_param) = common.signature.this_parameter.clone() {
                        let t = verifier.verify_type_expression(&this_param.type_annotation)?.unwrap_or(host.any_type());
                        activation.set_this(Some(host.factory().create_this_object(&t)));
                    } else if !slot.is_static() && (fn_parent.is::<ClassType>() || fn_parent.is::<EnumType>()) {
                        activation.set_this(Some(host.factory().create_this_object(&fn_parent)));
                    } else {
                        // Inherit "this" type
                        let super_act = verifier.scope().search_activation();
                        let super_this_type = super_act.and_then(|a| a.this().map(|this| this.static_type(&verifier.host)));
                        activation.set_this(Some(host.factory().create_this_object(&super_this_type.unwrap_or(host.any_type()))));
                    }

                    let partials1 = VerifierFunctionPartials::new(&activation, &loc);
                    verifier.function_definition_partials.set(NodeAsKey(common.clone()), partials1.clone());
                    partials = Some(partials1);
                }

                // Definition partials (2)
                let partials = partials.unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                // Verify parameter bindings
                let mut params: Vec<Rc<SemanticFunctionTypeParameter>> = vec![];
                let mut last_param_kind = ParameterKind::Required;        
                if partials.params().is_none() {
                    let internal_ns = kscope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

                    for param_node in &common.signature.parameters {
                        match param_node.kind {
                            ParameterKind::Required => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Optional => {
                                let param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.invalidation_entity());
                                } else {
                                    param_type = host.any_type();
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init;
                                if let Some(init1) = verifier.cached_var_init.get(&NodeAsKey(pattern.clone())) {
                                    init = init1.clone();
                                } else {
                                    init = verifier.imp_coerce_exp(param_node.default_value.as_ref().unwrap(), &param_type)?.unwrap_or(host.invalidation_entity());
                                    verifier.cached_var_init.insert(NodeAsKey(pattern.clone()), init.clone());
                                    if !init.is::<InvalidationEntity>() && !init.static_type(&host).is::<Constant>() {
                                        verifier.add_verify_error(&param_node.default_value.as_ref().unwrap().location(), WhackDiagnosticKind::EntityIsNotAConstant, diagarg![]);
                                    }
                                }
        
                                if last_param_kind.may_be_followed_by(param_node.kind) {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, &param_node.destructuring.destructuring, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                            ParameterKind::Rest => {
                                let mut param_type;
                                if let Some(type_annot) = param_node.destructuring.type_annotation.as_ref() {
                                    param_type = verifier.verify_type_expression(type_annot)?.unwrap_or(host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]));
                                    if param_type.array_element_type(&host)?.is_none() {
                                        verifier.add_verify_error(&type_annot.location(), WhackDiagnosticKind::RestParameterMustBeArray, diagarg![]);
                                        param_type = host.array_type().defer()?.apply_type(&host, &host.array_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                                    }
                                } else {
                                    param_type = host.array_type_of_any()?;
                                }
        
                                let pattern = &param_node.destructuring.destructuring;
                                let init = verifier.cache_var_init(pattern, || host.factory().create_value(&param_type));
        
                                if last_param_kind.may_be_followed_by(param_node.kind) && last_param_kind != ParameterKind::Rest {
                                    loop {
                                        match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false) {
                                            Ok(_) => {
                                                break;
                                            },
                                            Err(DeferError(Some(VerifierPhase::Beta))) |
                                            Err(DeferError(Some(VerifierPhase::Delta))) |
                                            Err(DeferError(Some(VerifierPhase::Epsilon))) |
                                            Err(DeferError(Some(VerifierPhase::Omega))) => {},
                                            Err(DeferError(_)) => {
                                                return Err(DeferError(None));
                                            },
                                        }
                                    }
        
                                    params.push(Rc::new(SemanticFunctionTypeParameter {
                                        kind: param_node.kind,
                                        static_type: param_type.clone(),
                                    }));
        
                                    verifier.cached_var_init.remove(&NodeAsKey(pattern.clone()));
                                }
                            },
                        }
                        last_param_kind = param_node.kind;
                    }

                    if params.len() != 1 {
                        verifier.add_verify_error(&loc, WhackDiagnosticKind::SetterMustTakeOneParameter, diagarg![]);
                        params.clear();
                        params.push(Rc::new(SemanticFunctionTypeParameter {
                            kind: ParameterKind::Required,
                            static_type: verifier.host.any_type(),
                        }));
                    }
        
                    partials.set_params(Some(params));
                }
        
                // Result type
                if let Some(result_annot) = common.signature.result_type.as_ref() {
                    if partials.result_type().is_none() {
                        let result_type = verifier.verify_type_expression(result_annot)?.unwrap_or(host.invalidation_entity());
                        if result_type != verifier.host.void_type() {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::SetterMustReturnVoid, diagarg![]);
                        }
                        partials.set_result_type(Some(host.void_type()));
                    }
                } else if partials.result_type().is_none() {
                    verifier.add_warning(&loc, WhackDiagnosticKind::ReturnValueHasNoTypeDeclaration, diagarg![]);
                    partials.set_result_type(Some(host.void_type()));
                }

                // Set signature
                let signature;
                if partials.signature().is_none() {
                    let result_type = partials.result_type().unwrap();
                    let signature1 = host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type);
                    partials.set_signature(Some(signature1.clone()));
                    signature = signature1;
                } else {
                    signature = partials.signature().unwrap();
                }
                slot.set_signature(&signature);

                // "override"
                let marked_override = Attribute::find_override(&defn.attributes).is_some();

                // Do not allow shadowing properties in base classes if not marked "override".
                if !marked_override {
                    let name = verifier.host.factory().create_qname(&ns, name.0.clone());
                    verifier.ensure_not_shadowing_definition(&loc, &fn_out, &fn_parent, &name);
                }

                // Restore scope
                verifier.set_scope(&kscope);

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                Err(DeferError(None))
            },
            VerifierPhase::Delta => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Database
                let host = verifier.host.clone();

                // Definition location
                let loc = name.1.clone();

                // Virtual slot
                let virtual_var = slot.of_virtual_slot(&verifier.host).unwrap();
                
                // Ensure the setter takes the correct data type
                if slot.signature(&verifier.host).params().get(0).unwrap().static_type != virtual_var.static_type(&verifier.host) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::SetterMustTakeDataType, diagarg![virtual_var.static_type(&verifier.host)]);
                }

                // Override if marked "override"
                if slot.is_overriding() {
                    match MethodOverride(&host).override_method(&slot, &verifier.scope().concat_open_ns_set_of_scope_chain()) {
                        Ok(_) => {},
                        Err(MethodOverrideError::Defer) => {
                            return Err(DeferError(None));
                        },
                        Err(MethodOverrideError::IncompatibleOverride { expected_signature, actual_signature }) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::IncompatibleOverride, diagarg![expected_signature.clone(), actual_signature.clone()]);
                        },
                        Err(MethodOverrideError::MustOverrideAMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::MustOverrideAMethod, diagarg![]);
                        },
                        Err(MethodOverrideError::OverridingFinalMethod) => {
                            verifier.add_verify_error(&loc, WhackDiagnosticKind::OverridingFinalMethod, diagarg![]);
                        },
                    }
                }

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                // Retrieve method slot
                let slot = verifier.host.node_mapping().get(drtv).unwrap();

                // Retrieve activation
                let activation = slot.activation().unwrap();

                // FunctionCommon
                let common = defn.common.clone();

                // Save scope
                let kscope = verifier.scope();

                // Definition partials
                let partials = verifier.function_definition_partials.get(&NodeAsKey(common.clone())).unwrap();

                // Enter scope
                verifier.inherit_and_enter_scope(&activation);

                FunctionCommonSubverifier::verify_function_definition_common(verifier, &common, &partials)?;

                // Restore scope
                verifier.set_scope(&kscope);

                // Finish
                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_package_concat_drtv(verifier: &mut Subverifier, drtv: &Rc<Directive>, pckgcat: &PackageConcatDirective) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        let host = verifier.host.clone();
        let alias_or_pckg = host.lazy_node_mapping(drtv, || {
            match &pckgcat.import_specifier {
                ImportSpecifier::Identifier(name) => {
                    let name_loc = name.1.clone();

                    // Initially unresolved if deferred;
                    // resolve any unresolved form in Beta phase.
                    let mut resolvee = host.unresolved_entity();
                    let pckg = host.factory().create_package(pckgcat.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
                    match pckg.properties(&host).get_in_ns_set_or_any_public_ns(&open_ns_set, &name.0) {
                        Ok(Some(resolvee1)) => {
                            Unused(&host).mark_used(&resolvee1);
                            resolvee = resolvee1;
                        },
                        Ok(None) => {},
                        Err(AmbiguousReferenceError(name)) => {
                            verifier.add_verify_error(&name_loc, WhackDiagnosticKind::AmbiguousReference, diagarg![name]);
                            resolvee = host.invalidation_entity();
                        },
                    }

                    let Some(public_ns) = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Public) else {
                        return host.invalidation_entity();
                    };
                    let qname = host.factory().create_qname(&public_ns, name.0.clone());
                    let mut alias = host.factory().create_alias(qname.clone(), resolvee);
                    alias.set_location(Some(drtv.location()));

                    // Define the alias, handling any conflict.
                    let mut out_names = verifier.scope().search_hoist_scope().properties(&host);
                    if let Some(prev) = out_names.get(&qname) {
                        alias = verifier.handle_definition_conflict(&prev, &alias);
                    } else {
                        out_names.set(qname, alias.clone());
                    }

                    alias
                },
                ImportSpecifier::Wildcard(_) => {
                    let pckg = host.factory().create_package(pckgcat.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    let scope = verifier.scope().search_hoist_scope();
                    if !scope.is::<PackageScope>() {
                        return host.invalidation_entity();
                    }
                    scope.package().package_concats().push(pckg.clone());
                    pckg
                },
                ImportSpecifier::Recursive(_) => {
                    let pckg = host.factory().create_package(pckgcat.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    let scope = verifier.scope().search_hoist_scope();
                    if !scope.is::<PackageScope>() {
                        return host.invalidation_entity();
                    }

                    let out_pckg = scope.package();

                    // Concatenate packages recursively, however
                    // ensure the packages to be concatenated are not
                    // circular.
                    if out_pckg.is_package_self_referential(&pckg) {
                        let err_loc = pckgcat.package_name[0].1.combine_with(pckgcat.package_name.last().unwrap().1.clone());
                        verifier.add_verify_error(&err_loc, WhackDiagnosticKind::ConcatenatingSelfReferentialPackage, diagarg![]);
                        return host.invalidation_entity();
                    }
                    let recursive_pckgs = pckg.list_packages_recursively();
                    scope.package().package_concats().extend(recursive_pckgs);

                    pckg
                },
            }
        });
        let resolved_alias = alias_or_pckg.is::<Alias>() && !alias_or_pckg.alias_of().is::<UnresolvedEntity>();
        if alias_or_pckg.is::<InvalidationEntity>() || resolved_alias {
            verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            // In Beta, resolve the alias, or ensure
            // the concatenated package is non-empty.
            VerifierPhase::Beta => {
                match &pckgcat.import_specifier {
                    ImportSpecifier::Identifier(name) => {
                        let name_loc = name.1.clone();
                        let pckg = host.factory().create_package(pckgcat.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
                        match pckg.properties(&host).get_in_ns_set_or_any_public_ns(&open_ns_set, &name.0) {
                            Ok(Some(resolvee)) => {
                                Unused(&host).mark_used(&resolvee);
                                alias_or_pckg.set_alias_of(&resolvee);
                            },
                            Ok(None) => {
                                verifier.add_verify_error(&pckgcat.package_name[0].1.combine_with(name.1.clone()), WhackDiagnosticKind::ImportOfUndefined, diagarg![
                                    format!("{}.{}", pckgcat.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join("."), name.0)]);
                                alias_or_pckg.set_alias_of(&host.invalidation_entity());
                            },
                            Err(AmbiguousReferenceError(name)) => {
                                verifier.add_verify_error(&name_loc, WhackDiagnosticKind::AmbiguousReference, diagarg![name]);
                                alias_or_pckg.set_alias_of(&host.invalidation_entity());
                            },
                        }
                    },
                    ImportSpecifier::Wildcard(_) => {
                        // Check for empty package (including concatenations) to report a warning.
                        if alias_or_pckg.is_empty_package(&host) {
                            verifier.add_verify_error(&pckgcat.package_name[0].1.combine_with(pckgcat.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![pckgcat.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                    ImportSpecifier::Recursive(_) => {
                        // Check for empty package recursively (including concatenations) to report a warning.
                        if alias_or_pckg.is_empty_package_recursive(&host) {
                            verifier.add_verify_error(&pckgcat.package_name[0].1.combine_with(pckgcat.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![pckgcat.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                }

                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_config_drtv(verifier: &mut Subverifier, drtv: &Rc<Directive>, cfgdrtv: &ConfigurationDirective) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }
        let host = verifier.host.clone();
        let concatenated_name = format!("{}::{}", cfgdrtv.namespace.0, cfgdrtv.constant_name.0);
        let cval = host.lazy_node_mapping(drtv, || {
            let loc = cfgdrtv.namespace.1.combine_with(cfgdrtv.constant_name.1.clone());
            if let Some(cdata) = verifier.host.config_constants().get(&concatenated_name) {
                let cval = ExpSubverifier::eval_config_constant(verifier, &loc, concatenated_name, cdata).unwrap_or(host.invalidation_entity());
                if !(cval.is::<BooleanConstant>() || cval.is::<InvalidationEntity>()) {
                    verifier.add_verify_error(&loc, WhackDiagnosticKind::NotABooleanConstant, diagarg![]);
                    return host.invalidation_entity();
                }
                cval
            } else {
                verifier.add_verify_error(&loc, WhackDiagnosticKind::CannotResolveConfigConstant, diagarg![concatenated_name.clone()]);
                host.invalidation_entity()
            }
        });

        if cval.is::<InvalidationEntity>() || !cval.boolean_value() {
            verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
            return Ok(());
        }

        // Do not just resolve the directive; if it is a block,
        // resolve it without creating a block scope for it.
        if let Directive::Block(block) = cfgdrtv.directive.as_ref() {
            Self::verify_directives(verifier, &block.directives)
        } else {
            Self::verify_directive(verifier, &cfgdrtv.directive)
        }
    }

    fn verify_use_ns_ns(verifier: &mut Subverifier, exp: &Rc<Expression>) -> Result<(), DeferError> {
        if let Expression::Sequence(seq) = exp.as_ref() {
            Self::verify_use_ns_ns(verifier, &seq.left)?;
            Self::verify_use_ns_ns(verifier, &seq.right)?;
            return Ok(());
        }
        let Some(cval) = verifier.verify_expression(exp, &default())? else {
            return Ok(());
        };
        if !cval.is::<NamespaceConstant>() {
            verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::NotANamespaceConstant, diagarg![]);
            return Ok(());
        }
        let ns = cval.referenced_ns();
        verifier.scope().open_ns_set().push(ns);
        Ok(())
    }

    fn verify_import_directive(verifier: &mut Subverifier, drtv: &Rc<Directive>, impdrtv: &ImportDirective) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        // Import alias
        if impdrtv.alias.is_some() {
            return Self::verify_import_alias_directive(verifier, drtv, impdrtv);
        }

        let host = verifier.host.clone();
        let imp = host.lazy_node_mapping(drtv, || {
            match &impdrtv.import_specifier {
                ImportSpecifier::Identifier(_) => {
                    // Initially unresolved import; resolve it in Beta phase.
                    host.factory().create_package_property_import(&host.unresolved_entity(), Some(drtv.location()))
                },
                ImportSpecifier::Wildcard(_) => {
                    let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    host.factory().create_package_wildcard_import(&pckg, Some(drtv.location()))
                },
                ImportSpecifier::Recursive(_) => {
                    let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    host.factory().create_package_recursive_import(&pckg, Some(drtv.location()))
                },
            }
        });

        match phase {
            VerifierPhase::Alpha => {
                // Mark unused
                Unused(&verifier.host).add(&imp);

                // Contribute to import list
                verifier.scope().search_hoist_scope().import_list().push(imp);

                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                match &impdrtv.import_specifier {
                    ImportSpecifier::Identifier(name) => {
                        let name_loc = name.1.clone();

                        // Resolve a property import
                        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
                        let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                        match pckg.properties(&host).get_in_ns_set_or_any_public_ns(&open_ns_set, &name.0) {
                            Ok(Some(prop)) => {
                                Unused(&host).mark_used(&prop);
                                imp.set_property(&prop);
                            },
                            Ok(None) => {
                                verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(name.1.clone()), WhackDiagnosticKind::ImportOfUndefined, diagarg![
                                    format!("{}.{}", impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join("."), name.0)]);

                                imp.set_property(&host.invalidation_entity());
                            },
                            Err(AmbiguousReferenceError(name)) => {
                                verifier.add_verify_error(&name_loc, WhackDiagnosticKind::AmbiguousReference, diagarg![name]);

                                imp.set_property(&host.invalidation_entity());
                            },
                        }
                    },
                    ImportSpecifier::Wildcard(_) => {
                        // Check for empty package (including concatenations) to report a warning.
                        if imp.package().is_empty_package(&host) {
                            verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(impdrtv.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                    ImportSpecifier::Recursive(_) => {
                        // Check for empty package, recursively, (including concatenations) to report
                        // a warning.
                        if imp.package().is_empty_package_recursive(&host) {
                            verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(impdrtv.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                }

                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_import_alias_directive(verifier: &mut Subverifier, drtv: &Rc<Directive>, impdrtv: &ImportDirective) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }
        let alias_name = impdrtv.alias.as_ref().unwrap();
        let host = verifier.host.clone();

        let internal_ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();
        let alias_qname = host.factory().create_qname(&internal_ns, alias_name.0.clone());

        let mut alias = host.lazy_node_mapping(drtv, || {
            let alias;
            match &impdrtv.import_specifier {
                ImportSpecifier::Identifier(_) => {
                    // Initially unresolved import; resolve it in Beta phase.
                    alias = host.factory().create_alias(alias_qname.clone(), host.unresolved_entity());
                },
                ImportSpecifier::Wildcard(_) => {
                    let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    let imp = host.factory().create_package_wildcard_import(&pckg, None);
                    alias = host.factory().create_alias(alias_qname.clone(), imp);
                },
                ImportSpecifier::Recursive(_) => {
                    let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                    let imp = host.factory().create_package_recursive_import(&pckg, None);
                    alias = host.factory().create_alias(alias_qname.clone(), imp);
                },
            }
            alias.set_location(Some(alias_name.1.clone()));
            alias
        });

        if alias.is::<InvalidationEntity>() {
            verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
            return Ok(());
        }

        match phase {
            VerifierPhase::Alpha => {
                // Mark unused
                Unused(&verifier.host).add(&alias);

                // Define the alias, handling any conflict.
                let mut out_names = verifier.scope().search_hoist_scope().properties(&host);
                if let Some(prev) = out_names.get(&alias_qname) {
                    alias = verifier.handle_definition_conflict(&prev, &alias);
                    host.node_mapping().set(drtv, Some(alias));
                } else {
                    out_names.set(alias_qname, alias);
                }

                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Resolve property or make sure an aliased package is not empty.

                match &impdrtv.import_specifier {
                    ImportSpecifier::Identifier(name) => {
                        let name_loc = name.1.clone();

                        // Resolve a property import
                        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
                        let pckg = host.factory().create_package(impdrtv.package_name.iter().map(|name| name.0.as_str()).collect::<Vec<_>>());
                        match pckg.properties(&host).get_in_ns_set_or_any_public_ns(&open_ns_set, &name.0) {
                            Ok(Some(prop)) => {
                                Unused(&host).mark_used(&prop);
                                alias.set_alias_of(&prop);
                            },
                            Ok(None) => {
                                verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(name.1.clone()), WhackDiagnosticKind::ImportOfUndefined, diagarg![
                                    format!("{}.{}", impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join("."), name.0)]);

                                alias.set_alias_of(&host.invalidation_entity());
                            },
                            Err(AmbiguousReferenceError(name)) => {
                                verifier.add_verify_error(&name_loc, WhackDiagnosticKind::AmbiguousReference, diagarg![name]);

                                alias.set_alias_of(&host.invalidation_entity());
                            },
                        }
                    },
                    ImportSpecifier::Wildcard(_) => {
                        // Check for empty package (including concatenations) to report a warning.
                        if alias.alias_of().package().is_empty_package(&host) {
                            verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(impdrtv.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                    ImportSpecifier::Recursive(_) => {
                        // Check for empty package, recursively, (including concatenations) to report
                        // a warning.
                        if alias.alias_of().package().is_empty_package_recursive(&host) {
                            verifier.add_verify_error(&impdrtv.package_name[0].1.combine_with(impdrtv.package_name.last().unwrap().1.clone()),
                                WhackDiagnosticKind::EmptyPackage,
                                diagarg![impdrtv.package_name.iter().map(|name| name.0.clone()).collect::<Vec<_>>().join(".")]);
                        }
                    },
                }

                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_config_subdirective(verifier: &mut Subverifier, drtv: &Rc<Directive>) -> Result<(), DeferError> {
        match drtv.as_ref() {
            Directive::Block(block) => {
                Self::verify_directives(verifier, &block.directives)
            },
            Directive::IfStatement(ifstmt) => {
                let Ok(cval) = verifier.verify_expression(&ifstmt.test, &default()) else {
                    verifier.add_verify_error(&ifstmt.test.location(), WhackDiagnosticKind::ReachedMaximumCycles, diagarg![]);
                    return Ok(());
                };
                let Some(cval) = cval else {
                    return Ok(());
                };
                if !cval.is::<BooleanConstant>() {
                    verifier.host.node_mapping().set(&ifstmt.test, None);
                    verifier.add_verify_error(&ifstmt.test.location(), WhackDiagnosticKind::NotABooleanConstant, diagarg![]);
                    return Ok(());
                }
                let bv = cval.boolean_value();
                if bv {
                    Self::verify_config_subdirective(verifier, &ifstmt.consequent)
                } else {
                    if let Some(alt) = &ifstmt.alternative {
                        Self::verify_config_subdirective(verifier, alt)
                    } else {
                        Ok(())
                    }
                }
            },
            _ => panic!(),
        }
    }

    pub fn verify_block(verifier: &mut Subverifier, block: &Rc<Block>) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_block_phase(block, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }
        let host = verifier.host.clone();
        let scope = host.lazy_node_mapping(block, || {
            host.factory().create_scope()
        });
        verifier.inherit_and_enter_scope(&scope);
        let any_defer = Self::verify_directives(verifier, &block.directives).is_err();
        verifier.exit_scope();
        if any_defer {
            Err(DeferError(None))
        } else {
            verifier.set_block_phase(block, VerifierPhase::Finished);
            Ok(())
        }
    }
}