use crate::ns::*;

/// # Phases
/// 
/// - `Alpha`
/// - `Beta`
/// - `Delta`
/// - `Epsilon`
/// - `Omega`
pub(crate) struct DestructuringDeclarationSubverifier;

impl DestructuringDeclarationSubverifier {
    /// Verifies a pattern.
    ///
    /// `init` may be a value, an `InvalidationEntity` or an `UnresolvedEntity`
    pub fn verify_pattern(verifier: &mut Subverifier, pattern: &Rc<Expression>, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        match pattern.as_ref() {
            Expression::QualifiedIdentifier(id) =>
                Self::verify_identifier_pattern(verifier, pattern, id, init, read_only, output, ns, parent, is_external, mark_used),
            Expression::ObjectInitializer(literal) =>
                Self::verify_object_pattern(verifier, pattern, literal, init, read_only, output, ns, parent, is_external, mark_used),
            Expression::ArrayLiteral(literal) =>
                Self::verify_array_pattern(verifier, pattern, literal, init, read_only, output, ns, parent, is_external, mark_used),
            Expression::Unary(e) => {
                if e.operator == Operator::NonNull {
                    Self::verify_non_null_pattern(verifier, pattern, e, init, read_only, output, ns, parent, is_external, mark_used)
                } else {
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }

    fn verify_identifier_pattern(verifier: &mut Subverifier, pattern: &Rc<Expression>, id: &QualifiedIdentifier, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        let mut slot = verifier.host.node_mapping().get(pattern);
        let mut slot_just_init = false;
        if slot.is_none() {
            let name = verifier.host.factory().create_qname(ns, id.to_identifier_name_or_asterisk().unwrap().0);
            let slot1 = verifier.host.factory().create_variable_slot(&name, read_only, &verifier.host.unresolved_entity());
            slot1.set_location(Some(id.location.clone()));
            slot1.set_parent(Some(parent.clone()));
            slot1.set_is_external(is_external);

            // Push variable as slot to codegen information
            if parent.is::<ClassType>() && !parent.is_external() && *output == parent.prototype(&verifier.host) {
                parent.codegen_slot_vars().push(slot1.clone());
            }

            // Ensure definition is not shadowing another definition
            verifier.ensure_not_shadowing_definition(&pattern.location(), output, parent, &name);

            // Define variable property
            if let Some(prev) = output.get(&name) {
                slot = Some(verifier.handle_definition_conflict(&prev, &slot1));
            } else {
                if !mark_used {
                    Unused(&verifier.host).add_nominal(&slot1);
                }
                output.set(name, slot1.clone());
                slot = Some(slot1);
            }

            verifier.host.node_mapping().set(pattern, slot.clone());

            slot_just_init = true;
        }

        let slot = slot.unwrap();
        if !slot.is::<VariableSlot>() {
            return Ok(());
        }

        let phase = verifier.phase_of_entity.get(&slot).cloned();
        if phase.is_none() && !slot_just_init {
            return Ok(());
        }

        let phase = phase.unwrap_or(VerifierPhase::Alpha);
        verifier.phase_of_entity.insert(slot.clone(), phase);

        match phase {
            VerifierPhase::Alpha => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Beta);
                Err(DeferError(Some(VerifierPhase::Beta)))
            },
            VerifierPhase::Beta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Delta);
                Err(DeferError(Some(VerifierPhase::Delta)))
            },
            VerifierPhase::Delta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Epsilon);
                Err(DeferError(Some(VerifierPhase::Epsilon)))
            },
            VerifierPhase::Epsilon => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Omega);
                Err(DeferError(Some(VerifierPhase::Omega)))
            },
            VerifierPhase::Omega => {
                // Assign a type if unresolved
                if slot.static_type(&verifier.host).is::<UnresolvedEntity>() {
                    init.defer()?;
                    slot.set_static_type(init.static_type(&verifier.host).defer()?);
                }

                if init.is::<Constant>() {
                    slot.set_var_constant(Some(init.clone()));
                }

                verifier.phase_of_entity.remove(&slot);

                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_array_pattern(verifier: &mut Subverifier, pattern: &Rc<Expression>, literal: &ArrayLiteral, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        let mut slot = verifier.host.node_mapping().get(pattern);
        let mut slot_just_init = false;
        if slot.is_none() {
            let name = verifier.host.empty_empty_qname();
            let slot1 = verifier.host.factory().create_variable_slot(&name, read_only, &verifier.host.unresolved_entity());
            slot1.set_parent(Some(parent.clone()));
            slot1.set_is_external(is_external);
            slot = Some(slot1);
            verifier.host.node_mapping().set(pattern, slot.clone());

            slot_just_init = true;
        }

        let slot = slot.unwrap();

        let phase = verifier.phase_of_entity.get(&slot).cloned();
        if phase.is_none() && !slot_just_init {
            return Ok(());
        }

        let phase = phase.unwrap_or(VerifierPhase::Alpha);
        verifier.phase_of_entity.insert(slot.clone(), phase);

        match phase {
            VerifierPhase::Alpha => {
                let mut rest_loc: Option<Location> = None;
                let mut i: usize = 0;
                let mut rest_i: usize = 0;
                for elem in &literal.elements {
                    match elem {
                        Element::Expression(subpat) => {
                            if let Err(DeferError(subphase)) = Self::verify_pattern(verifier, subpat, &verifier.host.unresolved_entity(), read_only, output, ns, parent, is_external, mark_used) {
                                assert_eq!(subphase, Some(VerifierPhase::Beta));
                            }
                        },
                        Element::Rest((restpat, loc)) => {
                            if rest_loc.is_some() {
                                verifier.add_verify_error(loc, WhackDiagnosticKind::UnexpectedRest, diagarg![]);
                            }
                            rest_i = i;
                            rest_loc = Some(loc.clone());
                            if let Err(DeferError(subphase)) = Self::verify_pattern(verifier, restpat, &verifier.host.unresolved_entity(), read_only, output, ns, parent, is_external, mark_used) {
                                assert_eq!(subphase, Some(VerifierPhase::Beta));
                            }
                        },
                        Element::Elision => {},
                    }
                    i += 1;
                }
                if rest_loc.is_some() && rest_i != i - 1 {
                    verifier.add_verify_error(&rest_loc.unwrap(), WhackDiagnosticKind::UnexpectedRest, diagarg![]);
                }
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Beta);
                Err(DeferError(Some(VerifierPhase::Beta)))
            },
            VerifierPhase::Beta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Delta);
                Err(DeferError(Some(VerifierPhase::Delta)))
            },
            VerifierPhase::Delta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Epsilon);
                Err(DeferError(Some(VerifierPhase::Epsilon)))
            },
            VerifierPhase::Epsilon => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Omega);
                Err(DeferError(Some(VerifierPhase::Omega)))
            },
            VerifierPhase::Omega => {
                init.defer()?;
                let init_st = init.static_type(&verifier.host).defer()?;
                let init_st_esc = init_st.escape_of_non_nullable();

                // Assign a type if unresolved
                if slot.static_type(&verifier.host).is::<UnresolvedEntity>() {
                    slot.set_static_type(init_st.clone());
                }

                // Verify Vector.<T> in omega phase
                if let Some(elem_type) = init_st_esc.vector_element_type(&verifier.host)? {
                    Self::verify_vector_array_pattern_omega(verifier, literal, &slot, &init_st_esc, &elem_type, read_only, output, ns, parent, is_external, mark_used)
                // Verify Array.<T> in omega phase
                } else if let Some(elem_type) = init_st_esc.array_element_type(&verifier.host)? {
                    Self::verify_array_array_pattern_omega(verifier, literal, &slot, &init_st_esc, &elem_type, read_only, output, ns, parent, is_external, mark_used)
                // Verify tuple in omega phase
                } else if init_st_esc.is::<TupleType>() {
                    Self::verify_tuple_array_pattern_omega(verifier, literal, &slot, &init_st_esc, read_only, output, ns, parent, is_external, mark_used)
                // Verify * or Object in omega phase
                } else if [verifier.host.any_type(), verifier.host.object_type().defer()?].contains(&init_st_esc) {
                    Self::verify_untyped_array_pattern_omega(verifier, literal, &slot, read_only, output, ns, parent, is_external, mark_used)
                // Invalidation in omega phase
                } else {
                    Self::verify_invalidation_array_pattern_omega(verifier, literal, &slot, read_only, output, ns, parent, is_external, mark_used)
                }
            },
            _ => panic!(),
        }
    }

    fn verify_vector_array_pattern_omega(verifier: &mut Subverifier, literal: &ArrayLiteral, patslot: &Entity, vector_type: &Entity, elem_type: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        for elem in &literal.elements {
            match elem {
                Element::Expression(subpat) => {
                    Self::verify_pattern(verifier, subpat, &verifier.host.factory().create_value(elem_type), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Rest((restpat, _)) => {
                    Self::verify_pattern(verifier, restpat, &verifier.host.factory().create_value(vector_type), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Elision => {},
            }
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }

    fn verify_array_array_pattern_omega(verifier: &mut Subverifier, literal: &ArrayLiteral, patslot: &Entity, array_type: &Entity, elem_type: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        for elem in &literal.elements {
            match elem {
                Element::Expression(subpat) => {
                    Self::verify_pattern(verifier, subpat, &verifier.host.factory().create_value(elem_type), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Rest((restpat, _)) => {
                    Self::verify_pattern(verifier, restpat, &verifier.host.factory().create_value(array_type), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Elision => {},
            }
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }

    fn verify_tuple_array_pattern_omega(verifier: &mut Subverifier, literal: &ArrayLiteral, patslot: &Entity, tuple_type: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        let elem_types = tuple_type.element_types();
        let mut i: usize = 0;
        let mut rest_found = false;

        for elem in &literal.elements {
            match elem {
                Element::Expression(subpat) => {
                    if rest_found || i >= elem_types.length() {
                        Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                    } else {
                        let elem_type = elem_types.get(i).unwrap();
                        Self::verify_pattern(verifier, subpat, &verifier.host.factory().create_value(&elem_type), read_only, output, ns, parent, is_external, mark_used)?;
                    }
                },
                Element::Rest((restpat, _)) => {
                    let array_type_of_any = verifier.host.array_type_of_any()?;
                    rest_found = true;
                    Self::verify_pattern(verifier, restpat, &verifier.host.factory().create_value(&array_type_of_any), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Elision => {},
            }
            i += 1;
        }

        if i > elem_types.length() && !rest_found {
            verifier.add_verify_error(&literal.location, WhackDiagnosticKind::ArrayLengthNotEqualsTupleLength, diagarg![tuple_type.clone()]);
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }

    fn verify_untyped_array_pattern_omega(verifier: &mut Subverifier, literal: &ArrayLiteral, patslot: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used:bool) -> Result<(), DeferError> {
        for elem in &literal.elements {
            match elem {
                Element::Expression(subpat) => {
                    Self::verify_pattern(verifier, subpat, &verifier.host.factory().create_value(&verifier.host.any_type()), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Rest((restpat, _)) => {
                    Self::verify_pattern(verifier, restpat, &verifier.host.factory().create_value(&verifier.host.any_type()), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Elision => {},
            }
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }

    fn verify_invalidation_array_pattern_omega(verifier: &mut Subverifier, literal: &ArrayLiteral, patslot: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        for elem in &literal.elements {
            match elem {
                Element::Expression(subpat) => {
                    Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Rest((restpat, _)) => {
                    Self::verify_pattern(verifier, restpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                },
                Element::Elision => {},
            }
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }

    fn verify_non_null_pattern(verifier: &mut Subverifier, pattern: &Rc<Expression>, literal: &UnaryExpression, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        let mut slot = verifier.host.node_mapping().get(pattern);
        let mut slot_just_init = false;
        if slot.is_none() {
            let name = verifier.host.empty_empty_qname();
            let slot1 = verifier.host.factory().create_variable_slot(&name, read_only, &verifier.host.unresolved_entity());
            slot1.set_parent(Some(parent.clone()));
            slot1.set_is_external(is_external);
            slot = Some(slot1);
            verifier.host.node_mapping().set(pattern, slot.clone());

            slot_just_init = true;
        }

        let slot = slot.unwrap();

        let phase = verifier.phase_of_entity.get(&slot).cloned();
        if phase.is_none() && !slot_just_init {
            return Ok(());
        }

        let phase = phase.unwrap_or(VerifierPhase::Alpha);
        verifier.phase_of_entity.insert(slot.clone(), phase);

        match phase {
            VerifierPhase::Alpha => {
                // Verify subpattern
                if let Err(DeferError(subphase)) = Self::verify_pattern(verifier, &literal.expression, &verifier.host.unresolved_entity(), read_only, output, ns, parent, is_external, mark_used) {
                    assert_eq!(subphase, Some(VerifierPhase::Beta));
                }

                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Beta);
                Err(DeferError(Some(VerifierPhase::Beta)))
            },
            VerifierPhase::Beta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Delta);
                Err(DeferError(Some(VerifierPhase::Delta)))
            },
            VerifierPhase::Delta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Epsilon);
                Err(DeferError(Some(VerifierPhase::Epsilon)))
            },
            VerifierPhase::Epsilon => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Omega);
                Err(DeferError(Some(VerifierPhase::Omega)))
            },
            VerifierPhase::Omega => {
                init.defer()?;
                let init_st = init.static_type(&verifier.host).defer()?;
                let init_st_esc = init_st.escape_of_nullable();

                // Assign a type if unresolved
                if slot.static_type(&verifier.host).is::<UnresolvedEntity>() {
                    slot.set_static_type(init_st.clone());
                }

                let init_st_is_non_null =
                        init_st.includes_null(&verifier.host)?
                    ||  init_st.includes_undefined(&verifier.host)?;

                let init_st_esc_is_non_null =
                        init_st_esc.includes_null(&verifier.host)?
                    ||  init_st_esc.includes_undefined(&verifier.host)?;

                let init_st_non_null = if init_st_esc_is_non_null {
                    init_st_esc.clone()
                } else {
                    verifier.host.factory().create_non_nullable_type(&init_st_esc)
                };

                // Verify subpattern
                Self::verify_pattern(verifier, &literal.expression, &verifier.host.factory().create_value(&init_st_non_null), read_only, output, ns, parent, is_external, mark_used)?;

                // Report warning
                if init_st_is_non_null {
                    verifier.add_warning(&literal.location, WhackDiagnosticKind::ReferenceIsAlreadyNonNullable, diagarg![]);
                }

                verifier.phase_of_entity.remove(&slot);

                Ok(())
            },
            _ => panic!(),
        }
    }

    fn verify_object_pattern(verifier: &mut Subverifier, pattern: &Rc<Expression>, literal: &ObjectInitializer, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        let mut slot = verifier.host.node_mapping().get(pattern);
        let mut slot_just_init = false;
        if slot.is_none() {
            let name = verifier.host.empty_empty_qname();
            let slot1 = verifier.host.factory().create_variable_slot(&name, read_only, &verifier.host.unresolved_entity());
            slot1.set_parent(Some(parent.clone()));
            slot1.set_is_external(is_external);
            slot = Some(slot1);
            verifier.host.node_mapping().set(pattern, slot.clone());

            slot_just_init = true;
        }

        let slot = slot.unwrap();

        let phase = verifier.phase_of_entity.get(&slot).cloned();
        if phase.is_none() && !slot_just_init {
            return Ok(());
        }

        let phase = phase.unwrap_or(VerifierPhase::Alpha);
        verifier.phase_of_entity.insert(slot.clone(), phase);

        match phase {
            VerifierPhase::Alpha => {
                Self::verify_object_pattern_alpha(verifier, literal, &slot, read_only, output, ns, parent, is_external, mark_used)
            },
            VerifierPhase::Beta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Delta);
                Err(DeferError(Some(VerifierPhase::Delta)))
            },
            VerifierPhase::Delta => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Epsilon);
                Err(DeferError(Some(VerifierPhase::Epsilon)))
            },
            VerifierPhase::Epsilon => {
                verifier.phase_of_entity.insert(slot.clone(), VerifierPhase::Omega);
                Err(DeferError(Some(VerifierPhase::Omega)))
            },
            VerifierPhase::Omega => {
                Self::verify_object_pattern_omega(verifier, literal, &slot, init, read_only, output, ns, parent, is_external, mark_used)
            },
            _ => panic!(),
        }
    }

    fn verify_object_pattern_alpha(verifier: &mut Subverifier, literal: &ObjectInitializer, patslot: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        // Verify fields
        for field in &literal.fields {
            match field.as_ref() {
                InitializerField::Field { name, non_null: _, value: subpat } => {
                    // DeclarativeFieldDestructuringResolution
                    let resolution = verifier.host.factory().create_declarative_field_destructuring_resolution();
                    verifier.host.node_mapping().set(field, Some(resolution.clone()));

                    if let Some(subpat) = subpat {
                        if let Err(DeferError(subphase)) = Self::verify_pattern(verifier, subpat, &verifier.host.unresolved_entity(), read_only, output, ns, parent, is_external, mark_used) {
                            assert_eq!(subphase, Some(VerifierPhase::Beta));
                        }
                    } else {
                        let Some(shorthand) = field.shorthand().and_then(|id| {
                            if let QualifiedIdentifierIdentifier::Id(id) = &id.id {
                                Some(id.clone())
                            } else {
                                None
                            }
                        }) else {
                            verifier.add_syntax_error(&name.1, WhackDiagnosticKind::UnexpectedFieldNameInDestructuring, diagarg![]);
                            continue;
                        };

                        resolution.set_var_slot(Some(Self::verify_shorthand_of_object_pattern_alpha(verifier, shorthand, read_only, output, ns, parent, is_external, mark_used)));
                    }
                },
                InitializerField::Rest((restpat, loc)) => {
                    verifier.add_verify_error(loc, WhackDiagnosticKind::UnexpectedRest, diagarg![]);
                    if let Err(DeferError(subphase)) = Self::verify_pattern(verifier, restpat, &verifier.host.unresolved_entity(), read_only, output, ns, parent, is_external, mark_used) {
                        assert_eq!(subphase, Some(VerifierPhase::Beta));
                    }
                },
            }
        }

        verifier.phase_of_entity.insert(patslot.clone(), VerifierPhase::Beta);
        Err(DeferError(Some(VerifierPhase::Beta)))
    }

    fn verify_shorthand_of_object_pattern_alpha(verifier: &mut Subverifier, shorthand: (String, Location), read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Entity {
        let name = verifier.host.factory().create_qname(ns, shorthand.0.clone());
        let slot = verifier.host.factory().create_variable_slot(&name, read_only, &verifier.host.unresolved_entity());
        slot.set_location(Some(shorthand.1.clone()));
        slot.set_parent(Some(parent.clone()));
        slot.set_is_external(is_external);

        // Push variable as slot to codegen information
        if parent.is::<ClassType>() && !parent.is_external() && *output == parent.prototype(&verifier.host) {
            parent.codegen_slot_vars().push(slot.clone());
        }

        // Ensure definition is not shadowing another definition
        verifier.ensure_not_shadowing_definition(&shorthand.1, output, parent, &name);

        if let Some(prev) = output.get(&name) {
            verifier.handle_definition_conflict(&prev, &slot)
        } else {
            if !mark_used {
                Unused(&verifier.host).add_nominal(&slot);
            }
            output.set(name, slot.clone());
            slot
        }
    }

    fn verify_object_pattern_omega(verifier: &mut Subverifier, literal: &ObjectInitializer, patslot: &Entity, init: &Entity, read_only: bool, output: &mut Names, ns: &Entity, parent: &Entity, is_external: bool, mark_used: bool) -> Result<(), DeferError> {
        init.defer()?;
        let init_st = init.static_type(&verifier.host).defer()?;

        // Verify fields
        for field in &literal.fields {
            match field.as_ref() {
                InitializerField::Field { name, non_null, value: subpat } => {
                    // Fields have an already attached DeclarativeFieldDestructuringResolution.
                    // A field is considered already verified if `field_reference()` is `Some`.
                    let resolution = verifier.host.node_mapping().get(field).unwrap();
                    if resolution.field_reference().is_some() {
                        continue;
                    }

                    let qn: Option<(Option<Entity>, PropertyLookupKey)>;

                    match &name.0 {
                        FieldName::Identifier(id) => {
                            qn = ExpSubverifier::verify_qualified_identifier(verifier, id)?;
                        },
                        FieldName::Brackets(exp) |
                        FieldName::NumericLiteral(exp) |
                        FieldName::StringLiteral(exp) => {
                            let val = verifier.verify_expression(exp, &default())?;
                            qn = Some((None, PropertyLookupKey::Computed(val.unwrap_or(verifier.host.invalidation_entity()))));
                        },
                    }

                    if qn.is_none() {
                        if let Some(var_slot) = resolution.var_slot() {
                            var_slot.set_static_type(verifier.host.invalidation_entity());
                        }
                        if let Some(subpat) = subpat {
                            Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                        }
                        resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                        continue;
                    }

                    let name_loc = &name.1;

                    let (qual, key) = qn.unwrap();

                    let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
                    let r = PropertyLookup(&verifier.host).lookup_in_object(&init, &open_ns_set, qual, &key, false);
                    if r.is_err() {
                        match r.unwrap_err() {
                            PropertyLookupError::AmbiguousReference(name) => {
                                if let Some(var_slot) = resolution.var_slot() {
                                    var_slot.set_static_type(verifier.host.invalidation_entity());
                                }
                                if let Some(subpat) = subpat {
                                    Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                                }
                                resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                                verifier.add_verify_error(name_loc, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                                continue;
                            },
                            PropertyLookupError::Defer => {
                                return Err(DeferError(None));
                            },
                            PropertyLookupError::VoidBase => {
                                if let Some(var_slot) = resolution.var_slot() {
                                    var_slot.set_static_type(verifier.host.invalidation_entity());
                                }
                                if let Some(subpat) = subpat {
                                    Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                                }
                                resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                                verifier.add_verify_error(name_loc, WhackDiagnosticKind::AccessOfVoid, diagarg![]);
                                continue;
                            },
                            PropertyLookupError::NullableObject { .. } => {
                                if let Some(var_slot) = resolution.var_slot() {
                                    var_slot.set_static_type(verifier.host.invalidation_entity());
                                }
                                if let Some(subpat) = subpat {
                                    Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                                }
                                resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                                verifier.add_verify_error(name_loc, WhackDiagnosticKind::AccessOfNullable, diagarg![]);
                                continue;
                            },
                        }
                    }
                    let r = r.unwrap();
                    if r.is_none() {
                        if let Some(var_slot) = resolution.var_slot() {
                            var_slot.set_static_type(verifier.host.invalidation_entity());
                        }
                        if let Some(subpat) = subpat {
                            Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                        }
                        resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                        verifier.add_verify_error(name_loc, WhackDiagnosticKind::UndefinedPropertyWithStaticType, diagarg![key.local_name().unwrap(), init_st.clone()]);
                        continue;
                    }
                    let r = r.unwrap();

                    // Post-processing
                    let postval = verifier.reference_post_processing(r, &default())?;
                    if let Some(mut postval) = postval {
                        if *non_null {
                            postval = verifier.host.factory().create_non_null_value(&postval)?;
                        }

                        if let Some(var_slot) = resolution.var_slot() {
                            var_slot.set_static_type(postval.static_type(&verifier.host));
                        }
                        if let Some(subpat) = subpat {
                            Self::verify_pattern(verifier, subpat, &postval, read_only, output, ns, parent, is_external, mark_used)?;
                        }
                        resolution.set_field_reference(Some(postval));
                    } else {
                        if let Some(var_slot) = resolution.var_slot() {
                            var_slot.set_static_type(verifier.host.invalidation_entity());
                        }
                        if let Some(subpat) = subpat {
                            Self::verify_pattern(verifier, subpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                        }
                        resolution.set_field_reference(Some(verifier.host.invalidation_entity()));
                    }
                },
                InitializerField::Rest((restpat, _)) => {
                    Self::verify_pattern(verifier, restpat, &verifier.host.invalidation_entity(), read_only, output, ns, parent, is_external, mark_used)?;
                },
            }
        }

        verifier.phase_of_entity.remove(&patslot);

        Ok(())
    }
}