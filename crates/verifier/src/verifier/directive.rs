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
            Directive::EnumDefinition(defn) => {
                Self::verify_enum_defn(verifier, drtv, defn)
            },
            Directive::InterfaceDefinition(defn) => {
                Self::verify_interface_defn(verifier, drtv, defn)
            },
            Directive::TypeDefinition(defn) => {
                Self::verify_type_defn(verifier, drtv, defn)
            },
            Directive::NamespaceDefinition(defn) => {
                Self::verify_namespace_defn(verifier, drtv, defn)
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
            // switch-type
            Directive::SwitchTypeStatement(swstmt) => {
                Self::verify_switch_type_stmt(verifier, drtv, swstmt)
            },
            Directive::DoStatement(dostmt) => {
                Self::verify_directive(verifier, &dostmt.body)
            },
            Directive::WhileStatement(whilestmt) => {
                Self::verify_directive(verifier, &whilestmt.body)
            },
            Directive::ForStatement(forstmt) => {
                Self::verify_for_stmt(verifier, drtv, forstmt)
            },
            Directive::ForInStatement(forstmt) => {
                Self::verify_for_in_stmt(verifier, drtv, forstmt)
            },
            Directive::WithStatement(withstmt) => {
                Self::verify_directive(verifier, &withstmt.body)
            },
            Directive::TryStatement(trystmt) => {
                Self::verify_try_stmt(verifier, drtv, trystmt)
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

    fn verify_try_stmt(verifier: &mut Subverifier, _drtv: &Rc<Directive>, trystmt: &TryStatement) -> Result<(), DeferError> {
        let mut any_defer = Self::verify_block(verifier, &trystmt.block).is_err();
        for catch_clause in &trystmt.catch_clauses {
            let parameter = &catch_clause.parameter;

            // scope
            let block_scope = verifier.host.lazy_node_mapping(&catch_clause.block, || {
                verifier.host.factory().create_scope()
            });
            let internal_ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

            // initialiser value
            let mut init: Option<Entity> = None;

            // verify type annotation
            if let Some(type_annot) = parameter.type_annotation.as_ref() {
                let t = verifier.verify_type_expression(type_annot)?.unwrap_or(verifier.host.any_type());
                init = Some(verifier.host.factory().create_value(&t));
            }

            let init = init.unwrap_or(verifier.host.factory().create_value(&verifier.host.any_type()));

            loop {
                match DestructuringDeclarationSubverifier::verify_pattern(verifier, &parameter.destructuring, &init, false, &mut block_scope.properties(&verifier.host), &internal_ns, &block_scope, false) {
                    Ok(_) => {
                        break;
                    },
                    Err(DeferError(_)) => {
                        return Err(DeferError(None));
                    },
                }
            }

            let r = Self::verify_block(verifier, &catch_clause.block).is_err();
            any_defer = any_defer || r;
        }
        if let Some(finally_clause) = trystmt.finally_clause.as_ref() {
            let r = Self::verify_block(verifier, &finally_clause.block).is_err();
            any_defer = any_defer || r;
        }
        if any_defer { Err(DeferError(None)) } else { Ok(()) }
    }

    fn verify_switch_type_stmt(verifier: &mut Subverifier, _drtv: &Rc<Directive>, swstmt: &SwitchTypeStatement) -> Result<(), DeferError> {
        let mut any_defer = false;
        for case in &swstmt.cases {
            // declare parameter
            if let Some(parameter) = case.parameter.as_ref() {
                // scope
                let block_scope = verifier.host.lazy_node_mapping(&case.block, || {
                    verifier.host.factory().create_scope()
                });
                let internal_ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

                // initialiser value
                let mut init: Option<Entity> = None;

                // verify type annotation
                if let Some(type_annot) = parameter.type_annotation.as_ref() {
                    let t = verifier.verify_type_expression(type_annot)?.unwrap_or(verifier.host.any_type());
                    init = Some(verifier.host.factory().create_value(&t));
                }

                let init = init.unwrap_or(verifier.host.factory().create_value(&verifier.host.any_type()));

                loop {
                    match DestructuringDeclarationSubverifier::verify_pattern(verifier, &parameter.destructuring, &init, false, &mut block_scope.properties(&verifier.host), &internal_ns, &block_scope, false) {
                        Ok(_) => {
                            break;
                        },
                        Err(DeferError(_)) => {
                            return Err(DeferError(None));
                        },
                    }
                }
            }

            let r = Self::verify_block(verifier, &case.block).is_err();
            any_defer = any_defer || r;
        }
        if any_defer { Err(DeferError(None)) } else { Ok(()) }
    }

    fn verify_for_stmt(verifier: &mut Subverifier, drtv: &Rc<Directive>, forstmt: &ForStatement) -> Result<(), DeferError> {
        let host = verifier.host.clone();
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        let scope = verifier.host.lazy_node_mapping(drtv, || {
            verifier.host.factory().create_scope()
        });

        if let Some(ForInitializer::VariableDefinition(defn)) = forstmt.init.as_ref() {
            let internal_ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

            match phase {
                // Alpha
                VerifierPhase::Alpha => {
                    for binding in &defn.bindings {
                        // Verify identifier binding or destructuring pattern (alpha)
                        let _ = DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &verifier.host.unresolved_entity(), defn.kind.0 == VariableDefinitionKind::Const, &mut scope.properties(&host), &internal_ns, &scope, false);
                    }

                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                    return Err(DeferError(None));
                },
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
                    return Err(DeferError(None));
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
                    return Err(DeferError(None));
                },
                // Omega
                VerifierPhase::Epsilon => {
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

                        // Verify the identifier binding or destructuring pattern
                        DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &init, is_const, &mut scope.properties(&host), &internal_ns, &scope, false)?;

                        // Remove *init1* from "cached_var_init"
                        verifier.cached_var_init.remove(&ByAddress(binding.destructuring.destructuring.clone()));

                        // If there is no type annotation and initialiser is unspecified,
                        // then report a warning
                        if binding.destructuring.type_annotation.is_none() && binding.initializer.is_none() {
                            verifier.add_warning(&binding.destructuring.location, WhackDiagnosticKind::VariableHasNoTypeAnnotation, diagarg![]);
                        }

                        // If variable is marked constant, is not `[Embed]` and does not contain an initializer,
                        // then report an error
                        if is_const {
                            verifier.add_verify_error(&binding.destructuring.location, WhackDiagnosticKind::ConstantMustContainInitializer, diagarg![]);
                        }
                    }

                    // Finish
                    verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                },
                VerifierPhase::Omega => {},
                _ => panic!(),
            }
        }
        verifier.inherit_and_enter_scope(&scope);
        let r = Self::verify_directive(verifier, &forstmt.body);
        verifier.exit_scope();
        r
    }

    fn verify_for_in_stmt(verifier: &mut Subverifier, drtv: &Rc<Directive>, forstmt: &ForInStatement) -> Result<(), DeferError> {
        let host = verifier.host.clone();
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        let scope = verifier.host.lazy_node_mapping(drtv, || {
            verifier.host.factory().create_scope()
        });
        if let ForInBinding::VariableDefinition(ref defn) = &forstmt.left {
            let internal_ns = verifier.scope().search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

            match phase {
                VerifierPhase::Alpha => {
                    let binding = &defn.bindings[0];

                    // Verify pattern (alpha)
                    let _ = DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &verifier.host.unresolved_entity(), defn.kind.0 == VariableDefinitionKind::Const, &mut scope.properties(&host), &internal_ns, &scope, false);

                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                    return Err(DeferError(None));
                },
                VerifierPhase::Beta => {
                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Delta);
                    return Err(DeferError(None));
                },
                VerifierPhase::Delta => {
                    // Resolve object key-value types
                    let obj = verifier.verify_expression(&forstmt.right, &Default::default())?;
                    let mut kv_types = (host.any_type(), host.any_type());
                    let mut illegal_obj = false;
                    if let Some(obj) = obj.as_ref() {
                        let kv_types_1 = StatementSubverifier::for_in_kv_types(&host, obj)?;
                        if let Some(kv_types_1) = kv_types_1 {
                            kv_types = kv_types_1;
                        } else {
                            illegal_obj = true;
                        }
                    }
                    let mut expected_type = if forstmt.each { kv_types.1 } else { kv_types.0 };

                    let binding = &defn.bindings[0];

                    // Resolve type annotation
                    let mut illegal_annotated_type = false;
                    let mut k_exty = expected_type.clone();
                    if let Some(t_node) = binding.destructuring.type_annotation.as_ref() {
                        let t = verifier.verify_type_expression(t_node)?;
                        if let Some(t) = t {
                            // If the expected type is not * or Object, then
                            // the annotated type must be either:
                            // - equals to or base type of the expected type (non nullable)
                            // - the * type
                            // - the Object type (non nullable)
                            // - a number type if the expected type is a number type
                            let obj_t = host.object_type().defer()?;
                            if ![host.any_type(), obj_t.clone()].contains(&expected_type) {
                                let anty = t.escape_of_non_nullable();
                                let exty = expected_type.escape_of_non_nullable();

                                let eq = exty.is_equals_or_subtype_of(&anty, &host)?;
                                let any_or_obj = anty == host.any_type() || anty == obj_t;
                                let num = host.numeric_types()?.contains(&anty) && host.numeric_types()?.contains(&exty);

                                if !(eq || any_or_obj || num) {
                                    illegal_annotated_type = true;
                                    k_exty = exty;
                                }
                            }

                            expected_type = t;
                        }
                    }

                    let is_const = defn.kind.0 == VariableDefinitionKind::Const;

                    // Resolve destructuring pattern
                    let init = host.factory().create_value(&expected_type);
                    match DestructuringDeclarationSubverifier::verify_pattern(verifier, &binding.destructuring.destructuring, &init, is_const, &mut scope.properties(&host), &internal_ns, &scope, false) {
                        Ok(_) => {},
                        Err(DeferError(None)) => {
                            return Err(DeferError(None));
                        },
                        Err(DeferError(Some(VerifierPhase::Beta))) |
                        Err(DeferError(Some(VerifierPhase::Delta))) |
                        Err(DeferError(Some(VerifierPhase::Epsilon))) |
                        Err(DeferError(Some(VerifierPhase::Omega))) => {},
                        Err(DeferError(Some(_))) => panic!(),
                    }

                    if illegal_obj {
                        verifier.add_verify_error(&forstmt.right.location(), WhackDiagnosticKind::CannotIterateType, diagarg![obj.unwrap().static_type(&host)]);
                    }

                    if illegal_annotated_type {
                        let t_node = binding.destructuring.type_annotation.as_ref().unwrap();
                        verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::ExpectedToIterateType, diagarg![k_exty]);
                    }

                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                    return Err(DeferError(None));
                },
                VerifierPhase::Omega => {},
                _ => panic!(),
            }
        }
    verifier.inherit_and_enter_scope( &scope);
        let r = Self::verify_directive(verifier, &forstmt.body);
        verifier.exit_scope();
        r
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
                let mut class_entity = verifier.host.factory().create_class_type(name.clone(), &public_ns);
                class_entity.set_parent(Some(class_parent.clone()));
                class_entity.set_asdoc(defn.asdoc.clone());
                class_entity.set_location(Some(defn.name.1.clone()));
                let metadata = Attribute::find_metadata(&defn.attributes);
                for m in metadata.iter() {
                    // [RecordLike] meta-data
                    if m.name.0 == "RecordLike" {
                        class_entity.set_is_record_like_class(true);
                        class_entity.set_is_final(true);
                    // [whack_external] meta-data
                    } else if m.name.0 == "whack_external" {
                        let mut slots = 0usize;

                        // Require the `slots="NUMBER"` pair,
                        // defining the number of elements contained in the instance Array
                        // at runtime (always counts the CONSTRUCTOR and DYNAMIC
                        // PROPERTIES slots, therefore it is at least "2").
                        if let Some(entries) = m.entries.as_ref() {
                            let mut found_slots = false;
                            for entry in entries {
                                if let Some(k) = entry.key.as_ref() {
                                    if k.0 == "slots" {
                                        use std::str::FromStr;
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(v) => v.0.clone(),
                                            MetadataValue::IdentifierString(v) => v.0.clone(),
                                        };
                                        slots = usize::from_str(&val).unwrap_or(0);
                                        found_slots = true;
                                    // Handle the local option for reusing a local
                                    } else if k.0 == "local" {
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(v) => v.0.clone(),
                                            MetadataValue::IdentifierString(v) => v.0.clone(),
                                        };
                                        class_entity.set_codegen_local(Some(val));
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
                        class_entity.set_codegen_slots(slots);

                        // Mark as external
                        class_entity.set_is_external(true);
                    }
                }

                let event_metadata_list = metadata.iter().filter(|m| {
                    m.name.0 == "Event"
                }).collect::<Vec<_>>();

                // Pre-declare Event meta-data
                for m in event_metadata_list.iter() {
                    let mut name: Option<String> = None;
                    let mut bubbles: Option<bool> = None;

                    if let Some(entries) = m.entries.as_ref() {
                        for entry in entries {
                            if let Some((k, _)) = entry.key.as_ref() {
                                // Value
                                let val = match entry.value.as_ref() {
                                    MetadataValue::String(val) => val.0.clone(),
                                    MetadataValue::IdentifierString(val) => val.0.clone(),
                                };

                                // name="eventName" entry
                                if k == "name" {
                                    name = Some(val);
                                // bubbles="boolean" entry
                                } else if k == "bubbles" {
                                    bubbles = Some(val == "true");
                                }
                            }
                        }
                    }

                    if name.is_none() {
                        continue;
                    } else {
                        let name = name.unwrap();

                        // Contribute Event
                        class_entity.events().set(name.clone(), Event {
                            data_type: verifier.host.unresolved_entity(),
                            bubbles,
                            constant: None,
                        });
                    }
                }

                class_entity.metadata().extend(metadata);
                class_entity.set_is_static(Attribute::find_static(&defn.attributes).is_some());
                class_entity.set_is_dynamic(Attribute::find_dynamic(&defn.attributes).is_some());
                class_entity.set_is_abstract(Attribute::find_abstract(&defn.attributes).is_some());
                class_entity.set_is_final(Attribute::find_final(&defn.attributes).is_some());

                let is_the_object_class = name.namespace() == verifier.host.top_level_package().public_ns().unwrap() && name.local_name() == "Object";
                if is_the_object_class {
                    class_entity.set_extends_class(None);
                } else {
                    class_entity.set_extends_class(Some(verifier.host.unresolved_entity()));
                }

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

                // Map directive to class entity
                verifier.host.node_mapping().set(drtv, if class_entity.is::<ClassType>() { Some(class_entity.clone()) } else { None });

                // Create class block scope
                let block_scope = verifier.host.factory().create_class_scope(&class_entity);
                verifier.node_mapping().set(&defn.block, Some(block_scope.clone()));

                // Contribute private namespace to open namespace set
                block_scope.open_ns_set().push(class_entity.private_ns().unwrap());

                // Declare type parameters if specified in syntax
                if let Some(list) = defn.type_parameters.as_ref() {
                    let internal_ns = class_parent_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();
                    for type_param_node in list {
                        let name = verifier.host.factory().create_qname(&internal_ns, type_param_node.name.0.clone());
                        let type_param = verifier.host.factory().create_type_parameter_type(&name);

                        // Contribute type parameter
                        if class_entity.type_params().is_none() {
                            class_entity.set_type_params(Some(shared_array![]));
                        }
                        class_entity.type_params().unwrap().push(type_param.clone());

                        // Place type parameter into block scope
                        let type_alias = verifier.host.factory().create_alias(name.clone(), type_param.clone());
                        block_scope.properties(&verifier.host).set(name.clone(), type_alias);
                    }
                }

                // Enter class block scope and visit class block but DO NOT defer; then exit scope
                verifier.inherit_and_enter_scope(&block_scope);
                let _ = DirectiveSubverifier::verify_directives(verifier, &defn.block.directives);
                verifier.exit_scope();

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                Err(DeferError(None))
            },
            VerifierPhase::Beta => {
                // Database
                let host = verifier.host.clone();

                // Class entity
                let class_entity = host.node_mapping().get(drtv).unwrap();

                // Resolve the class inheritance (which class it extends)
                // (CONDITION: in case it is "unresolved" yet).
                if let Some(base_class) = class_entity.extends_class(&host) {
                    if base_class.is::<UnresolvedEntity>() {
                        if let Some(t_node) = defn.extends_clause.as_ref() {
                            let t = verifier.verify_type_expression(&t_node)?;
                            if let Some(t) = t {
                                if t.is_class_type_possibly_after_sub() {
                                    // Ensure extended class is not final
                                    if t.is_final() {
                                        verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::CannotExtendFinalClass, diagarg![t.clone()]);
                                    }

                                    // Ensure extended class is not self referential
                                    if class_entity == t || t.is_subtype_of(&class_entity, &host)? {
                                        verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::ExtendingSelfReferentialClass, diagarg![]);
                                        class_entity.set_extends_class(Some(host.object_type().defer()?));
                                    } else {
                                        // Contribute class to the list of known subclasses of t.
                                        t.known_subclasses().push(class_entity.clone());

                                        // Set extended class
                                        class_entity.set_extends_class(Some(t));
                                    }
                                } else {
                                    verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::NotAClass, diagarg![]);
                                    class_entity.set_extends_class(Some(host.object_type().defer()?));
                                }
                            } else {
                                class_entity.set_extends_class(Some(host.object_type().defer()?));
                            }
                        } else {
                            class_entity.set_extends_class(Some(host.object_type().defer()?));
                        }
                    }
                }

                let guard = verifier.class_defn_guard(drtv);

                // (GUARD: do not double this step)
                // Resolve the interface implements list,
                // contributing to the list of implemented interfaces of the class.
                if !guard.implements_list_done.get() {
                    if let Some(implements_list) = defn.implements_clause.as_ref() {
                        class_entity.implements(&host).clear();
                        class_entity.implements(&host).push(host.unresolved_entity());

                        let mut implements_t: Vec<Entity> = vec![];
                        for t_node in implements_list {
                            let t = verifier.verify_type_expression(&t_node)?;
                            if let Some(t) = t {
                                if t.is_interface_type_possibly_after_sub() {
                                    implements_t.push(t);
                                } else {
                                    verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::NotAnInterface, diagarg![]);
                                }
                            }
                        }

                        class_entity.implements(&host).clear();
                        for t in &implements_t {
                            if class_entity.implements(&host).index_of(t).is_none() { 
                                class_entity.implements(&host).push(t.clone());

                                // Count implementor
                                t.known_implementors().push(class_entity.clone());
                            }
                        }
                    }
                    guard.implements_list_done.set(true);
                }

                let mut about_to_defer = host.object_type().is::<UnresolvedEntity>();

                // If `is_record_like_class()` is true and the class is not a direct subclass
                // of `Object`, report a verify error and call `set_is_record_like_class(false)`.
                if !about_to_defer && class_entity.is_record_like_class() && class_entity.extends_class(&host).map(|b| b == host.object_type()).unwrap_or(true) {
                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::RecordLikeClassMustExtendObject, diagarg![]);
                    class_entity.set_is_record_like_class(false);
                }

                // Given all present `[Event]` meta-data
                if !guard.event_metadata_done.get() {
                    let metadata = Attribute::find_metadata(&defn.attributes);
                    let event_metadata_list = metadata.iter().filter(|m| {
                        m.name.0 == "Event"
                    }).collect::<Vec<_>>();

                    // Resolve the `type="Name"` pair for each meta-data into a local (but DEFER ONLY AT THE FINAL STEP if necessary.).
                    let mut type_list: Vec<Entity> = vec![];
                    let mut cancel = false;
                    'm: for m in event_metadata_list.iter() {
                        let mut found_type = false;
                        if let Some(entries) = m.entries.as_ref() {
                            for entry in entries {
                                if let Some((k, _)) = entry.key.as_ref() {
                                    // type="T" entry
                                    if k == "type" {
                                        // Value
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(val) => {
                                                (val.0.clone(), Location::with_offsets(&val.1.compilation_unit(), val.1.first_offset() + 1, val.1.last_offset() - 1))
                                            },
                                            MetadataValue::IdentifierString(val) => val.clone(),
                                        };

                                        // Parse type expression
                                        let tyexp = ParserFacade(&val.1.compilation_unit(), ParserOptions {
                                            byte_range: Some((val.1.first_offset(), val.1.last_offset())),
                                            ..default()
                                        }).parse_type_expression();

                                        // Verify
                                        let t = verifier.verify_type_expression(&tyexp);
                                        if let Ok(t) = t {
                                            let t = t.unwrap_or(host.object_type());
                                            about_to_defer = t.is::<UnresolvedEntity>();
                                            if about_to_defer {
                                                cancel = true;
                                                break 'm;
                                            }
                                            type_list.push(t);
                                        } else {
                                            about_to_defer = true;
                                            cancel = true;
                                            break 'm;
                                        }

                                        found_type = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !found_type {
                            // push Object
                            about_to_defer = host.object_type().is::<UnresolvedEntity>();
                            if about_to_defer {
                                cancel = true;
                                break 'm;
                            }
                            type_list.push(host.object_type());
                        }
                    }

                    if !cancel {
                        // Resolve every `[Event]` meta-data using
                        // the previous type locals, contributing events to the class.
                        'm: for i in 0..event_metadata_list.len() {
                            let m = event_metadata_list[i];
                            let mut name: Option<String> = None;
                            let m_type = &type_list[i];
                            let mut bubbles: Option<bool> = None;

                            if let Some(entries) = m.entries.as_ref() {
                                for entry in entries {
                                    if let Some((k, _)) = entry.key.as_ref() {
                                        // Value
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(val) => val.0.clone(),
                                            MetadataValue::IdentifierString(val) => val.0.clone(),
                                        };

                                        // name="eventName" entry
                                        if k == "name" {
                                            name = Some(val);
                                        // bubbles="boolean" entry
                                        } else if k == "bubbles" {
                                            bubbles = Some(val == "true");
                                        }
                                    }
                                }
                            }

                            if name.is_none() {
                                verifier.add_verify_error(&m.location, WhackDiagnosticKind::MalformedEventMetadata, diagarg![]);
                            } else {
                                let name = name.unwrap();
                                let mut constant: Option<Entity> = None;

                                // Resolve @eventType ASDoc tag
                                if let Some(asdoc) = m.asdoc.as_ref() {
                                    for tag in &asdoc.tags {
                                        if let AsdocTag::EventType(exp) = &tag.0 {
                                            let val = verifier.verify_expression(exp, &Default::default());
                                            if val.is_err() {
                                                cancel = true;
                                                about_to_defer = true;
                                                break 'm;
                                            }
                                            let val = val.unwrap();
                                            if val.is_none() {
                                                break;
                                            }
                                            let val = val.unwrap();

                                            if (val.is::<StaticReferenceValue>() || val.is::<PackageReferenceValue>()) && val.property().is::<VariableSlot>() {
                                                constant = Some(val.property());
                                            }

                                            break;
                                        }
                                    }
                                }

                                // Contribute Event
                                class_entity.events().set(name.clone(), Event {
                                    data_type: m_type.clone(),
                                    bubbles,
                                    constant,
                                });
                            }
                        }

                        if !cancel {
                            guard.event_metadata_done.set(true);
                        }
                    }
                }

                let block_scope = verifier.host.node_mapping().get(&defn.block).unwrap();

                // Contribute protected namespaces to open namespace set
                let mut c = Some(class_entity);
                let mut protected_ns_list: Vec<Entity> = vec![];
                let mut cancel_protected_ns = false;
                while let Some(c1) = c {
                    if c1.is::<UnresolvedEntity>() {
                        about_to_defer = true;
                        cancel_protected_ns = true;
                        break;
                    }
                    protected_ns_list.push(c1.protected_ns().unwrap());
                    protected_ns_list.push(c1.static_protected_ns().unwrap());
                    c = c1.extends_class(&verifier.host);
                }
                if !cancel_protected_ns {
                    for ns in &protected_ns_list {
                        block_scope.open_ns_set().push(ns.clone());
                    }
                }

                if !about_to_defer {
                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                }

                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                // Database
                let host = verifier.host.clone();

                // Class entity
                let class_entity = host.node_mapping().get(drtv).unwrap();

                let mut about_to_defer: bool;

                // Class block scope
                let block_scope = verifier.host.node_mapping().get(&defn.block).unwrap();

                // Enter class block scope, then visit class block
                // but DEFER ONLY AT THE FINAL STEP if necessary; then exit scope.
                verifier.inherit_and_enter_scope(&block_scope);
                about_to_defer = DirectiveSubverifier::verify_directives(verifier, &defn.block.directives).is_err();
                verifier.exit_scope();

                let guard = verifier.class_defn_guard(drtv);

                // Report a verify error for non overriden abstract methods
                // but DEFER ONLY AT THE FINAL STEP if necessary.
                if !guard.abstract_overrides_done.get() {
                    let list = MethodOverride(&host).abstract_methods_not_overriden(&class_entity, &block_scope.concat_open_ns_set_of_scope_chain());
                    if let Ok(list) = list {
                        for m in list.iter() {
                            if let Some(virtual_slot) = m.of_virtual_slot(&host) {
                                let mut is_getter = false;
                                if let Some(getter) = virtual_slot.getter(&host) {
                                    if m == &getter {
                                        is_getter = true;
                                    }
                                }
                                if is_getter {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::AbstractGetterMustBeOverriden, diagarg![m.name().to_string()]);
                                } else {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::AbstractSetterMustBeOverriden, diagarg![m.name().to_string()]);
                                }
                            } else {
                                verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::AbstractMethodMustBeOverriden, diagarg![m.name().to_string()]);
                            }
                        }
                        guard.abstract_overrides_done.set(true);
                    } else {
                        about_to_defer = true;
                    }
                }

                // If the base class contains a non-empty constructor,
                // that (sub)class must define a constructor
                if !guard.default_constructor_done.get() {
                    let base_class = class_entity.extends_class(&host);
                    if base_class.as_ref().map(|c| c.is::<UnresolvedEntity>()).unwrap_or(true) {
                        if base_class.is_none() {
                            guard.default_constructor_done.set(true);
                        } else {
                            about_to_defer = true;
                        }
                    } else {
                        let ctor = base_class.unwrap().constructor_method(&host);
                        if let Some(ctor) = ctor {
                            let sig = ctor.signature(&host);
                            if sig.is::<UnresolvedEntity>() {
                                about_to_defer = true;
                            } else {
                                let has_required = sig.params().iter().any(|p| p.kind == ParameterKind::Required);
                                if has_required && class_entity.constructor_method(&host).is_none() {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::ClassMustDefineAConstructor, diagarg![]);
                                }
                                guard.default_constructor_done.set(true);
                            }
                        } else {
                            guard.default_constructor_done.set(true);
                        }
                    }
                }

                // Ensure RecordLike classes have an empty constructor.
                if !guard.record_like_ctor_done.get() {
                    if class_entity.is_record_like_class() {
                        if let Some(ctor) = class_entity.constructor_method(&host) {
                            let ctorsig = ctor.signature(&host);
                            if ctorsig.is::<UnresolvedEntity>() {
                                about_to_defer = true;
                            } else {
                                if ctorsig.params().length() != 0 {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::RecordLikeClassMustHaveEmptyConstructor, diagarg![]);
                                }
                                guard.record_like_ctor_done.set(true);
                            }
                        } else {
                            guard.record_like_ctor_done.set(true);
                        }
                    } else {
                        guard.record_like_ctor_done.set(true);
                    }
                }

                // Verify interface implementations but DEFER ONLY AT THE FINAL STEP if necessary.
                if !guard.interface_impl_done.get() {
                    let mut cancel_impl = false;
                    for itrfc in class_entity.implements(&host).iter() {
                        let logs = InterfaceImplement(&host).verify(&class_entity, &itrfc);
                        if logs.is_err() {
                            about_to_defer = true;
                            cancel_impl = true;
                            break;
                        }
                        let logs = logs.unwrap();
                        for log in logs {
                            match log {
                                InterfaceImplementationLog::MethodNotImplemented { name } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::MethodNotImplemented, diagarg![name]);
                                },
                                InterfaceImplementationLog::GetterNotImplemented { name } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::GetterNotImplemented, diagarg![name]);
                                },
                                InterfaceImplementationLog::SetterNotImplemented { name } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::SetterNotImplemented, diagarg![name]);
                                },
                                InterfaceImplementationLog::IncompatibleMethodSignature { name, expected_signature } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::IncompatibleMethodSignature, diagarg![name, expected_signature]);
                                },
                                InterfaceImplementationLog::IncompatibleGetterSignature { name, expected_signature } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::IncompatibleGetterSignature, diagarg![name, expected_signature]);
                                },
                                InterfaceImplementationLog::IncompatibleSetterSignature { name, expected_signature } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::IncompatibleSetterSignature, diagarg![name, expected_signature]);
                                },
                                InterfaceImplementationLog::PropertyMustBeMethod { name } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::PropertyMustBeMethod, diagarg![name]);
                                },
                                InterfaceImplementationLog::PropertyMustBeVirtual { name } => {
                                    verifier.add_verify_error(&defn.name.1, WhackDiagnosticKind::PropertyMustBeVirtual, diagarg![name]);
                                },
                            }
                        }
                    }

                    if !cancel_impl {
                        guard.interface_impl_done.set(true);
                    }
                }

                if about_to_defer {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            _ => panic!(),
        }
    }

    fn verify_enum_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &EnumDefinition) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            // Alpha
            VerifierPhase::Alpha => {
                // Determine the enum's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_never_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (enum_parent_scope, enum_parent, mut enum_out, ns) = defn_local.unwrap();

                let public_ns = enum_parent_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Public).unwrap();
                let name = verifier.host.factory().create_qname(&ns, defn.name.0.clone());
                let mut enum_entity = verifier.host.factory().create_enum_type(name.clone(), &public_ns);
                enum_entity.set_parent(Some(enum_parent.clone()));
                enum_entity.set_asdoc(defn.asdoc.clone());
                enum_entity.set_location(Some(defn.name.1.clone()));

                // Attach meta-data
                let metadata = Attribute::find_metadata(&defn.attributes);
                enum_entity.metadata().extend(metadata);

                // Attempt to define the enum partially;
                // or fail if a conflict occurs, therefore ignoring
                // this enum definition.
                if let Some(prev) = enum_out.get(&name) {
                    enum_entity = verifier.handle_definition_conflict(&prev, &enum_entity);
                } else {
                    Unused(&verifier.host).add_nominal(&enum_entity);
                    enum_out.set(name, enum_entity.clone());
                }
                if !enum_entity.is::<EnumType>() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map directive to enum entity
                verifier.host.node_mapping().set(drtv, if enum_entity.is::<EnumType>() { Some(enum_entity.clone()) } else { None });

                // Create enum block scope
                let block_scope = verifier.host.factory().create_class_scope(&enum_entity);
                verifier.node_mapping().set(&defn.block, Some(block_scope.clone()));

                // Contribute private namespace to open namespace set
                block_scope.open_ns_set().push(enum_entity.private_ns().unwrap());

                // Enter enum block scope
                verifier.inherit_and_enter_scope(&block_scope);

                // Process defining constants and mark them as in the finished phase.
                let mut counter: f64 = 0.0;
                for drtv in defn.block.directives.iter() {
                    if let Directive::VariableDefinition(defn) = drtv.as_ref() {
                        if Attribute::find_static(&defn.attributes).is_some() {
                            continue;
                        }
                        'b: for binding in defn.bindings.iter() {
                            if let Expression::QualifiedIdentifier(id) = binding.destructuring.destructuring.as_ref() {
                                let id = id.to_identifier_name_or_asterisk().unwrap();
                                let screaming_name = id.0.clone();
                                let mut string_name: Option<String> = None;
                                let mut value: Option<f64> = None;

                                if let Some(init) = binding.initializer.as_ref() {
                                    match init.as_ref() {
                                        Expression::StringLiteral(StringLiteral { ref value, .. }) => {
                                            string_name = Some(value.clone());
                                        },
                                        Expression::NumericLiteral(literal) => {
                                            value = Some(literal.parse_double(false).unwrap_or(0.0));
                                        },
                                        Expression::Unary(UnaryExpression { operator: op, ref expression, .. }) => {
                                            if *op == Operator::Negative {
                                                if let Expression::NumericLiteral(literal) = expression.as_ref() {
                                                    value = Some(literal.parse_double(true).unwrap_or(0.0));
                                                }
                                            } else {
                                                verifier.add_verify_error(&init.location(), WhackDiagnosticKind::IllegalEnumConstInit, diagarg![]);
                                                continue;
                                            }
                                        },
                                        Expression::ArrayLiteral(ArrayLiteral { ref elements, .. }) => {
                                            'elem: for elem in elements.iter() {
                                                match elem {
                                                    Element::Expression(ref exp) => {
                                                        match exp.as_ref() {
                                                            Expression::StringLiteral(StringLiteral { ref value, .. }) => {
                                                                string_name = Some(value.clone());
                                                            },
                                                            Expression::NumericLiteral(literal) => {
                                                                value = Some(literal.parse_double(false).unwrap_or(0.0));
                                                            },
                                                            Expression::Unary(UnaryExpression { operator: op, ref expression, .. }) => {
                                                                if *op == Operator::Negative {
                                                                    if let Expression::NumericLiteral(literal) = expression.as_ref() {
                                                                        value = Some(literal.parse_double(true).unwrap_or(0.0));
                                                                    }
                                                                } else {
                                                                    verifier.add_verify_error(&init.location(), WhackDiagnosticKind::IllegalEnumConstInit, diagarg![]);
                                                                    continue 'elem;
                                                                }
                                                            },
                                                            _ => {
                                                                verifier.add_verify_error(&init.location(), WhackDiagnosticKind::IllegalEnumConstInit, diagarg![]);
                                                                continue 'elem;
                                                            },
                                                        }
                                                    },
                                                    _ => {},
                                                }
                                            }
                                        },
                                        _ => {
                                            verifier.add_verify_error(&init.location(), WhackDiagnosticKind::IllegalEnumConstInit, diagarg![]);
                                            continue;
                                        },
                                    }
                                }

                                // Automatically convert screaming snake case
                                // into lowercase camel case.
                                if string_name.is_none() {
                                    let p = screaming_name.split('_').collect::<Vec<_>>();
                                    let mut new_string = String::new();
                                    new_string.push_str(&p[0].to_lowercase());
                                    if p.len() > 1 {
                                        for p1 in p[1..].iter() {
                                            if p1.len() == 1 {
                                                new_string.push_str(&p1.to_uppercase());
                                            } else if p1.len() > 1 {
                                                new_string.push_str(&p1.to_uppercase());
                                                new_string.push_str(&p1[1..].to_lowercase());
                                            }
                                        }
                                    }
                                    string_name = Some(new_string);
                                }

                                // Automatically count value.
                                if value.is_none() {
                                    value = Some(counter);
                                    counter += 1.0;
                                }

                                let string_name = string_name.unwrap();
                                let value = value.unwrap();

                                // Ensure string is not duplicate
                                if enum_entity.enum_member_number_mapping().has(&string_name) {
                                    verifier.add_verify_error(&binding.location(), WhackDiagnosticKind::DuplicateEnumString, diagarg![string_name.clone()]);
                                    continue;
                                }

                                // Ensure value is not duplicate
                                for (_, val1) in enum_entity.enum_member_number_mapping().borrow().iter() {
                                    if value == val1.force_double() {
                                        verifier.add_verify_error(&binding.location(), WhackDiagnosticKind::DuplicateEnumValue, diagarg![value.to_string()]);
                                        continue 'b;
                                    }
                                }

                                let name = verifier.host.factory().create_qname(&public_ns, screaming_name.clone());

                                // Ensure constant property is not duplicate
                                if enum_entity.properties(&verifier.host).has(&name) {
                                    verifier.add_verify_error(&binding.location(), WhackDiagnosticKind::DuplicateEnumConstant, diagarg![screaming_name.clone()]);
                                    continue 'b;
                                }

                                let const_slot = verifier.host.factory().create_variable_slot(&name, true, &enum_entity);
                                const_slot.set_parent(Some(enum_entity.clone()));
                                const_slot.set_location(Some(binding.destructuring.location.clone()));
                                const_slot.set_asdoc(defn.asdoc.clone());

                                // Attach meta-data
                                let metadata = Attribute::find_metadata(&defn.attributes);
                                const_slot.metadata().extend(metadata);

                                // Define constant property
                                enum_entity.properties(&verifier.host).set(name, const_slot.clone());

                                enum_entity.enum_member_number_mapping().set(string_name.clone(), Number::Number(value));
                                enum_entity.enum_member_slot_mapping().set(string_name, const_slot);
                            }
                        }
                        verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    }
                }

                // Visit enum block but DO NOT defer
                let _ = DirectiveSubverifier::verify_directives(verifier, &defn.block.directives);

                // Exit scope
                verifier.exit_scope();

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                return Err(DeferError(None));
            },
            VerifierPhase::Beta => {
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                let about_to_defer: bool;

                // Enum block scope
                let block_scope = verifier.host.node_mapping().get(&defn.block).unwrap();

                // Enter enum block scope, then visit enum block
                // but DEFER ONLY AT THE FINAL STEP if necessary; then exit scope.
                verifier.inherit_and_enter_scope(&block_scope);
                about_to_defer = DirectiveSubverifier::verify_directives(verifier, &defn.block.directives).is_err();
                verifier.exit_scope();

                if about_to_defer {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            _ => panic!(),
        }
    }

    fn verify_interface_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &InterfaceDefinition) -> Result<(), DeferError> {
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
                let (itrfc_parent_scope, itrfc_parent, mut itrfc_out, ns) = defn_local.unwrap();

                let name = verifier.host.factory().create_qname(&ns, defn.name.0.clone());
                let mut itrfc_entity = verifier.host.factory().create_interface_type(name.clone());
                itrfc_entity.set_parent(Some(itrfc_parent.clone()));
                itrfc_entity.set_asdoc(defn.asdoc.clone());
                itrfc_entity.set_location(Some(defn.name.1.clone()));
                let metadata = Attribute::find_metadata(&defn.attributes);
                for m in metadata.iter() {
                    // [whack_external] meta-data
                    if m.name.0 == "whack_external" {
                        // Mark as external
                        itrfc_entity.set_is_external(true);

                        // Detect local option
                        if let Some(entries) = m.entries.as_ref() {
                            for entry in entries {
                                if let Some(k) = entry.key.as_ref() {
                                    if k.0 == "local" {
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(v) => v.0.clone(),
                                            MetadataValue::IdentifierString(v) => v.0.clone(),
                                        };
                                        itrfc_entity.set_codegen_local(Some(val));
                                    }
                                }
                            }
                        }
                    }
                }
                itrfc_entity.metadata().extend(metadata);

                // Attempt to define the interface partially;
                // or fail if a conflict occurs, therefore ignoring
                // this interface definition.
                if let Some(prev) = itrfc_out.get(&name) {
                    itrfc_entity = verifier.handle_definition_conflict(&prev, &itrfc_entity);
                } else {
                    Unused(&verifier.host).add_nominal(&itrfc_entity);
                    itrfc_out.set(name, itrfc_entity.clone());
                }
                if !itrfc_entity.is::<InterfaceType>() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map directive to interface entity
                verifier.host.node_mapping().set(drtv, if itrfc_entity.is::<InterfaceType>() { Some(itrfc_entity.clone()) } else { None });

                // Create interface block scope
                let block_scope = verifier.host.factory().create_interface_scope(&itrfc_entity);
                verifier.node_mapping().set(&defn.block, Some(block_scope.clone()));

                // Declare type parameters if specified in syntax
                if let Some(list) = defn.type_parameters.as_ref() {
                    let internal_ns = itrfc_parent_scope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();
                    for type_param_node in list {
                        let name = verifier.host.factory().create_qname(&internal_ns, type_param_node.name.0.clone());
                        let type_param = verifier.host.factory().create_type_parameter_type(&name);

                        // Contribute type parameter
                        if itrfc_entity.type_params().is_none() {
                            itrfc_entity.set_type_params(Some(shared_array![]));
                        }
                        itrfc_entity.type_params().unwrap().push(type_param.clone());

                        // Place type parameter into block scope
                        let type_alias = verifier.host.factory().create_alias(name.clone(), type_param.clone());
                        block_scope.properties(&verifier.host).set(name.clone(), type_alias);
                    }
                }

                // Enter interface block scope and visit interface block but DO NOT defer; then exit scope
                verifier.inherit_and_enter_scope(&block_scope);
                for drtv in defn.block.directives.iter() {
                    if matches!(drtv.as_ref(), Directive::FunctionDefinition(_)) {
                        let _ = DirectiveSubverifier::verify_directive(verifier, drtv);
                    }
                }
                verifier.exit_scope();

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Beta);
                return Err(DeferError(None));
            },
            VerifierPhase::Beta => {
                // Database
                let host = verifier.host.clone();

                // Class entity
                let itrfc_entity = host.node_mapping().get(drtv).unwrap();

                let guard = verifier.itrfc_defn_guard(drtv);

                // (GUARD: do not double this step)
                // Resolve the interface extends list.
                if !guard.extends_list_done.get() {
                    if let Some(extends_list) = defn.extends_clause.as_ref() {
                        itrfc_entity.extends_interfaces(&host).clear();
                        itrfc_entity.extends_interfaces(&host).push(host.unresolved_entity());

                        let mut extends_t: Vec<Entity> = vec![];
                        for t_node in extends_list {
                            let t = verifier.verify_type_expression(&t_node)?;
                            if let Some(t) = t {
                                if t.is_interface_type_possibly_after_sub() {
                                    if t == itrfc_entity || t.is_subtype_of(&itrfc_entity, &host)? {
                                        verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::ExtendingSelfReferentialInterface, diagarg![]);
                                    } else {
                                        extends_t.push(t);
                                    }
                                } else {
                                    verifier.add_verify_error(&t_node.location(), WhackDiagnosticKind::NotAnInterface, diagarg![]);
                                }
                            }
                        }
                        itrfc_entity.extends_interfaces(&host).clear();
                        for t in &extends_t {
                            if itrfc_entity.extends_interfaces(&host).index_of(t).is_none() { 
                                itrfc_entity.extends_interfaces(&host).push(t.clone());
                            }
                        }
                    }
                    guard.extends_list_done.set(true);
                }

                let mut about_to_defer = host.object_type().is::<UnresolvedEntity>();

                // Given all present `[Event]` meta-data
                if !guard.event_metadata_done.get() {
                    let metadata = Attribute::find_metadata(&defn.attributes);
                    let event_metadata_list = metadata.iter().filter(|m| {
                        m.name.0 == "Event"
                    }).collect::<Vec<_>>();

                    // Resolve the `type="Name"` pair for each meta-data into a local (but DEFER ONLY AT THE FINAL STEP if necessary.).
                    let mut type_list: Vec<Entity> = vec![];
                    let mut cancel = false;
                    'm: for m in event_metadata_list.iter() {
                        let mut found_type = false;
                        if let Some(entries) = m.entries.as_ref() {
                            for entry in entries {
                                if let Some((k, _)) = entry.key.as_ref() {
                                    // type="T" entry
                                    if k == "type" {
                                        // Value
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(val) => {
                                                (val.0.clone(), Location::with_offsets(&val.1.compilation_unit(), val.1.first_offset() + 1, val.1.last_offset() - 1))
                                            },
                                            MetadataValue::IdentifierString(val) => val.clone(),
                                        };

                                        // Parse type expression
                                        let tyexp = ParserFacade(&val.1.compilation_unit(), ParserOptions {
                                            byte_range: Some((val.1.first_offset(), val.1.last_offset())),
                                            ..default()
                                        }).parse_type_expression();

                                        // Verify
                                        let t = verifier.verify_type_expression(&tyexp);
                                        if let Ok(t) = t {
                                            let t = t.unwrap_or(host.object_type());
                                            about_to_defer = t.is::<UnresolvedEntity>();
                                            if about_to_defer {
                                                cancel = true;
                                                break 'm;
                                            }
                                            type_list.push(t);
                                        } else {
                                            about_to_defer = true;
                                            cancel = true;
                                            break 'm;
                                        }

                                        found_type = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !found_type {
                            // push Object
                            about_to_defer = host.object_type().is::<UnresolvedEntity>();
                            if about_to_defer {
                                cancel = true;
                                break 'm;
                            }
                            type_list.push(host.object_type());
                        }
                    }

                    if !cancel {
                        // Resolve every `[Event]` meta-data using
                        // the previous type locals, contributing events to the class.
                        'm: for i in 0..event_metadata_list.len() {
                            let m = event_metadata_list[i];
                            let mut name: Option<String> = None;
                            let m_type = &type_list[i];
                            let mut bubbles: Option<bool> = None;

                            if let Some(entries) = m.entries.as_ref() {
                                for entry in entries {
                                    if let Some((k, _)) = entry.key.as_ref() {
                                        // Value
                                        let val = match entry.value.as_ref() {
                                            MetadataValue::String(val) => val.0.clone(),
                                            MetadataValue::IdentifierString(val) => val.0.clone(),
                                        };

                                        // name="eventName" entry
                                        if k == "name" {
                                            name = Some(val);
                                        // bubbles="boolean" entry
                                        } else if k == "bubbles" {
                                            bubbles = Some(val == "true");
                                        }
                                    }
                                }
                            }

                            if name.is_none() {
                                verifier.add_verify_error(&m.location, WhackDiagnosticKind::MalformedEventMetadata, diagarg![]);
                            } else {
                                let name = name.unwrap();
                                let mut constant: Option<Entity> = None;

                                // Resolve @eventType ASDoc tag
                                if let Some(asdoc) = m.asdoc.as_ref() {
                                    for tag in &asdoc.tags {
                                        if let AsdocTag::EventType(exp) = &tag.0 {
                                            let val = verifier.verify_expression(exp, &Default::default());
                                            if val.is_err() {
                                                cancel = true;
                                                about_to_defer = true;
                                                break 'm;
                                            }
                                            let val = val.unwrap();
                                            if val.is_none() {
                                                break;
                                            }
                                            let val = val.unwrap();

                                            if (val.is::<StaticReferenceValue>() || val.is::<PackageReferenceValue>()) && val.property().is::<VariableSlot>() {
                                                constant = Some(val.property());
                                            }

                                            break;
                                        }
                                    }
                                }

                                // Contribute Event
                                itrfc_entity.events().set(name.clone(), Event {
                                    data_type: m_type.clone(),
                                    bubbles,
                                    constant,
                                });
                            }
                        }

                        if !cancel {
                            guard.event_metadata_done.set(true);
                        }
                    }
                }

                if !about_to_defer {
                    // Next phase
                    verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                }

                Err(DeferError(None))
            },
            VerifierPhase::Omega => {
                let mut about_to_defer: bool = false;

                // Class block scope
                let block_scope = verifier.host.node_mapping().get(&defn.block).unwrap();

                // Enter interface block scope, then visit interface block
                // but DEFER ONLY AT THE FINAL STEP if necessary; then exit scope.
                verifier.inherit_and_enter_scope(&block_scope);
                for drtv in defn.block.directives.iter() {
                    if matches!(drtv.as_ref(), Directive::FunctionDefinition(_)) {
                        about_to_defer = DirectiveSubverifier::verify_directive(verifier, drtv).is_err() || about_to_defer;
                    }
                }
                verifier.exit_scope();

                if about_to_defer {
                    Err(DeferError(None))
                } else {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    Ok(())
                }
            },
            _ => panic!(),
        }
    }

    fn verify_type_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &TypeDefinition) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            // Alpha
            VerifierPhase::Alpha => {
                // Determine the type alias's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_never_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, t_alias_parent, mut t_alias_out, ns) = defn_local.unwrap();

                let name = verifier.host.factory().create_qname(&ns, defn.left.0.clone());
                let mut t_alias = verifier.host.factory().create_alias(name.clone(), verifier.host.unresolved_entity());
                t_alias.set_parent(Some(t_alias_parent.clone()));
                t_alias.set_location(Some(defn.left.1.clone()));

                // Attempt to define the type alias partially;
                // or fail if a conflict occurs, therefore ignoring
                // this type alias definition.
                if let Some(prev) = t_alias_out.get(&name) {
                    t_alias = verifier.handle_definition_conflict(&prev, &t_alias);
                } else {
                    Unused(&verifier.host).add_nominal(&t_alias);
                    t_alias_out.set(name, t_alias.clone());
                }
                if !t_alias.is::<Alias>() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                // Map directive to type alias entity
                verifier.host.node_mapping().set(drtv, if t_alias.is::<Alias>() { Some(t_alias.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                return Err(DeferError(None));
            },
            VerifierPhase::Omega => {
                // Database
                let host = verifier.host.clone();

                // Type alias
                let t_alias = host.node_mapping().get(drtv).unwrap();

                if t_alias.alias_of().is::<UnresolvedEntity>() {
                    let t = verifier.verify_type_expression(&defn.right)?.unwrap_or(verifier.host.any_type());
                    t_alias.set_alias_of(&t);
                }

                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_namespace_defn(verifier: &mut Subverifier, drtv: &Rc<Directive>, defn: &NamespaceDefinition) -> Result<(), DeferError> {
        let phase = verifier.lazy_init_drtv_phase(drtv, VerifierPhase::Alpha);
        if phase == VerifierPhase::Finished {
            return Ok(());
        }

        match phase {
            // Alpha
            VerifierPhase::Alpha => {
                // Determine the namespace's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, ns_alias_parent, mut ns_alias_out, ns) = defn_local.unwrap();

                let name = verifier.host.factory().create_qname(&ns, defn.left.0.clone());
                let mut ns_alias = verifier.host.factory().create_alias(name.clone(), verifier.host.unresolved_entity());
                ns_alias.set_parent(Some(ns_alias_parent.clone()));
                ns_alias.set_location(Some(defn.left.1.clone()));

                // Attempt to define the namespace alias partially;
                // or fail if a conflict occurs, therefore ignoring
                // this namespace definition.
                if let Some(prev) = ns_alias_out.get(&name) {
                    ns_alias = verifier.handle_definition_conflict(&prev, &ns_alias);
                } else {
                    Unused(&verifier.host).add_nominal(&ns_alias);
                    ns_alias_out.set(name, ns_alias.clone());

                    // Throw a verify error if the namespace conflicts with a
                    // configuration namespace.
                    let prefix = defn.left.0.clone() + "::";
                    for (name, _) in verifier.host.config_constants().borrow().iter() {
                        if name.starts_with(&prefix) {
                            verifier.add_verify_error(&defn.left.1, WhackDiagnosticKind::NamespaceConflictsWithConfigurationNs, diagarg![]);
                            break;
                        }
                    }
                }
                if !ns_alias.is::<Alias>() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }

                if let Some(r) = defn.right.as_ref() {
                    if let Expression::StringLiteral(literal) = r.as_ref() {
                        ns_alias.set_alias_of(&verifier.host.factory().create_user_ns(literal.value.clone()));
                    }
                }

                // Map directive to type alias entity
                verifier.host.node_mapping().set(drtv, if ns_alias.is::<Alias>() { Some(ns_alias.clone()) } else { None });

                // Next phase
                verifier.set_drtv_phase(drtv, VerifierPhase::Omega);
                return Err(DeferError(None));
            },
            VerifierPhase::Omega => {
                // Database
                let host = verifier.host.clone();

                // Type alias
                let ns_alias = host.node_mapping().get(drtv).unwrap();

                if ns_alias.alias_of().is::<UnresolvedEntity>() {
                    if let Some(r) = defn.right.as_ref() {
                        let val = verifier.verify_expression(r, &Default::default())?.unwrap_or(host.invalidation_entity());
                        if val.is::<StringConstant>() {
                            ns_alias.set_alias_of(&host.factory().create_user_ns(val.string_value()));
                        } else if val.is::<NamespaceConstant>() {
                            ns_alias.set_alias_of(&val.referenced_ns());
                        } else {
                            verifier.add_verify_error(&r.location(), WhackDiagnosticKind::NotANamespaceConstant, diagarg![]);
                            ns_alias.set_alias_of(&host.invalidation_entity());
                        }
                    } else {
                        // Create alias to a new internal namespace
                        ns_alias.set_alias_of(&host.factory().create_internal_ns(None));
                    }
                }

                // Determine the namespace's scope, parent, property destination, and namespace.
                let defn_local = Self::definition_local_maybe_static(verifier, &defn.attributes)?;
                if defn_local.is_err() {
                    verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                    return Ok(());
                }
                let (_, ns_alias_parent, ns_alias_out, ns) = defn_local.unwrap();

                let name = verifier.host.factory().create_qname(&ns, defn.left.0.clone());
                verifier.ensure_not_shadowing_definition(&defn.left.1, &ns_alias_out, &ns_alias_parent, &name);

                verifier.set_drtv_phase(drtv, VerifierPhase::Finished);
                Ok(())
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
            // [whack_external]
            defn.attributes.iter().find(|a| {
                if let Attribute::Metadata(m) = a { m.name.0 == "whack_external" } else { false }
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
                    // [whack_external]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "whack_external" } else { false }
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
                        let mut result_type = verifier.verify_type_expression(result_annot)?.unwrap_or(host.invalidation_entity());
                        if common.contains_await && result_type.promise_result_type(&host)?.is_none() {
                            let prom_t = host.promise_type().defer();
                            result_type = host.factory().create_type_after_substitution(&prom_t?, &shared_array![result_type]);
                        }
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
                    // [whack_external]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "whack_external" } else { false }
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
                    // [whack_external]
                    defn.attributes.iter().find(|a| {
                        if let Attribute::Metadata(m) = a { m.name.0 == "whack_external" } else { false }
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