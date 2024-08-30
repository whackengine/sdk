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