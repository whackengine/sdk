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