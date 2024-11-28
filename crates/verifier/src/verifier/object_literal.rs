use crate::ns::*;

pub(crate) struct ObjectLiteralSubverifier;

impl ObjectLiteralSubverifier {
    pub fn verify_object_initializer(verifier: &mut Subverifier, initializer: &ObjectInitializer, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        let context_type = context.context_type.clone().unwrap_or(verifier.host.any_type());
        context_type.defer()?;
        let context_type_esc = context_type.escape_of_nullable_or_non_nullable();

        let object_type = verifier.host.object_type().defer()?;

        verifier.host.string_type().defer()?;
        verifier.host.number_type().defer()?;
        verifier.host.boolean_type().defer()?;
        verifier.host.namespace_type().defer()?;

        if [verifier.host.any_type(), object_type].contains(&context_type_esc) {
            Self::verify_object_initializer_for_ecma_object(verifier, initializer)?;
        } else if context_type_esc.is_record_like_class() {
            Self::verify_object_initializer_for_record_like_class(verifier, initializer, &context_type_esc)?;
        } else {
            if !context_type_esc.is::<InvalidationEntity>() {
                verifier.add_verify_error(&initializer.location, WhackDiagnosticKind::UnexpectedObject, diagarg![]);
            }
            // Same as for * and Object; duplicating the call for now.
            Self::verify_object_initializer_for_ecma_object(verifier, initializer)?;
        }

        if context_type_esc.is::<InvalidationEntity>() {
            return Ok(Some(verifier.host.invalidation_entity()));
        }

        Ok(Some(verifier.host.factory().create_value(&context_type)))
    }

    fn verify_object_initializer_for_ecma_object(verifier: &mut Subverifier, initializer: &ObjectInitializer) -> Result<(), DeferError> {
        for field in &initializer.fields {
            match field.as_ref() {
                InitializerField::Rest((exp, _)) => {
                    verifier.verify_expression(exp, &default())?;
                },
                InitializerField::Field { name, value, .. } => {
                    if let Some(name) = field.shorthand() {
                        let fr = verifier.host.lazy_node_mapping(field, || verifier.host.factory().create_field_resolution());
                        fr.set_shorthand_resolution(Self::verify_initializer_shorthand(verifier, name)?);
                    } else {
                        if let FieldName::Brackets(exp) = &name.0 {
                            verifier.verify_expression(exp, &default())?;
                        }
                        verifier.verify_expression(value.as_ref().unwrap(), &default())?;
                    }
                },
            }
        }
        Ok(())
    }

    fn verify_object_initializer_for_record_like_class(verifier: &mut Subverifier, initializer: &ObjectInitializer, record_like_class: &Entity) -> Result<(), DeferError> {
        let mut missing = HashSet::<Entity>::new();
        for (_, entity) in record_like_class.prototype(&verifier.host).borrow().iter() {
            if entity.is::<VariableSlot>() && !entity.is_opt_variable_for_record_like_class(&verifier.host)? {
                entity.static_type(&verifier.host).defer()?;
                missing.insert(entity.clone());
            }
        }

        for field in &initializer.fields {
            match field.as_ref() {
                InitializerField::Rest((exp, _)) => {
                    verifier.imp_coerce_exp(exp, &record_like_class)?;
                    missing.clear();
                },
                InitializerField::Field { .. } => {
                    if let Some(name) = field.shorthand() {
                        let variable = Self::resolve_instance_variable(verifier, &record_like_class, &name)?;
                        if let Some(variable) = variable.clone() {
                            missing.remove(&variable);
                        }
                        let mut short_ref = Self::verify_initializer_shorthand(verifier, &name)?;
                        if let Some(short_ref_1) = short_ref.as_ref() {
                            if let Some(variable) = variable.as_ref() {
                                let variable_data_type = variable.static_type(&verifier.host);
                                variable_data_type.defer()?;
                                let coercion = ConversionMethods(&verifier.host).implicit(short_ref_1, &variable_data_type, false)?;
                                let Some(coercion) = coercion else {
                                    verifier.add_verify_error(&name.location, WhackDiagnosticKind::ImplicitCoercionToUnrelatedType, diagarg![short_ref_1.static_type(&verifier.host), variable_data_type]);
                                    #[allow(unused_assignments)] {
                                        short_ref = None;
                                    }
                                    continue;
                                };
                                short_ref = Some(coercion);
                            }
                        }
                        let fr = verifier.host.lazy_node_mapping(field, || verifier.host.factory().create_field_resolution());
                        fr.set_field_slot(variable);
                        fr.set_shorthand_resolution(short_ref);
                    } else {
                        Self::verify_non_shorthand_notation_for_record_like_class(field, verifier, &record_like_class, &mut missing)?;
                    }
                },
            }
        }

        for m in &missing {
            verifier.add_verify_error(&initializer.location, WhackDiagnosticKind::MustSpecifyOption, diagarg![m.name().to_string()]);
        }

        Ok(())
    }

    fn verify_non_shorthand_notation_for_record_like_class(field: &Rc<InitializerField>, verifier: &mut Subverifier, record_like_class: &Entity, missing: &mut HashSet<Entity>) -> Result<(), DeferError> {
        let InitializerField::Field { name, value, .. } = field.as_ref() else {
            panic!();
        };
        let value_exp = value.as_ref().unwrap();
        match &name.0 {
            FieldName::Brackets(exp) => {
                verifier.imp_coerce_exp(exp, &verifier.host.string_type())?;
                verifier.verify_expression(value_exp, &default())?;
                missing.clear();
            },
            FieldName::Identifier(name_1) => {
                let variable = Self::resolve_instance_variable(verifier, record_like_class, name_1)?;
                if let Some(variable) = variable.clone() {
                    missing.remove(&variable);
                }
                if let Some(variable) = variable {
                    let variable_data_type = variable.static_type(&verifier.host).defer()?;
                    verifier.imp_coerce_exp(value_exp, &variable_data_type)?;
                } else {
                    verifier.verify_expression(value_exp, &default())?;
                }
            },
            FieldName::StringLiteral(sl) => {
                let name_1 = verifier.verify_expression(sl, &default())?.unwrap().string_value();
                let variable = Self::resolve_instance_variable(verifier, record_like_class, &QualifiedIdentifier {
                    location: sl.location(),
                    attribute: false,
                    qualifier: None,
                    id: QualifiedIdentifierIdentifier::Id((name_1, sl.location())),
                })?;
                if let Some(variable) = variable.clone() {
                    missing.remove(&variable);
                }
                if let Some(variable) = variable {
                    let variable_data_type = variable.static_type(&verifier.host).defer()?;
                    verifier.imp_coerce_exp(value_exp, &variable_data_type)?;
                } else {
                    verifier.verify_expression(value_exp, &default())?;
                }
            },
            FieldName::NumericLiteral(_) => {
                verifier.verify_expression(value_exp, &default())?;
                verifier.add_verify_error(&name.1, WhackDiagnosticKind::UnexpectedFieldName, diagarg![]);
            },
        }
        Ok(())
    }

    fn verify_initializer_shorthand(verifier: &mut Subverifier, id: &QualifiedIdentifier) -> Result<Option<Entity>, DeferError> {
        let qn = ExpSubverifier::verify_qualified_identifier(verifier, id)?;
        if qn.is_none() {
            return Ok(None);
        }
        let (qual, key) = qn.unwrap();

        let r = verifier.scope().lookup_in_scope_chain(&verifier.host, qual, &key);
        if r.is_err() {
            match r.unwrap_err() {
                PropertyLookupError::AmbiguousReference(name) => {
                    verifier.add_verify_error(&id.location, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                    return Ok(None);
                },
                PropertyLookupError::Defer => {
                    return Err(DeferError(None));
                },
                PropertyLookupError::VoidBase => {
                    verifier.add_verify_error(&id.location, WhackDiagnosticKind::AccessOfVoid, diagarg![]);
                    return Ok(None);
                },
                PropertyLookupError::NullableObject { .. } => {
                    verifier.add_verify_error(&id.location, WhackDiagnosticKind::AccessOfNullable, diagarg![]);
                    return Ok(None);
                },
            }
        }
        let r = r.unwrap();
        if r.is_none() {
            verifier.add_verify_error(&id.location, WhackDiagnosticKind::UndefinedProperty, diagarg![key.local_name().unwrap()]);
            return Ok(None);
        }
        let r = r.unwrap();

        if r.is::<InvalidationEntity>() {
            return Ok(None);
        }

        // Mark local capture
        verifier.detect_local_capture(&r);

        // Post-processing
        verifier.reference_post_processing(r, &default())
    }

    fn resolve_instance_variable(verifier: &mut Subverifier, class: &Entity, id: &QualifiedIdentifier) -> Result<Option<Entity>, DeferError> {
        let qn = ExpSubverifier::verify_qualified_identifier(verifier, id)?;
        if qn.is_none() {
            return Ok(None);
        }
        let (qual, key) = qn.unwrap();

        let has_known_ns = qual.as_ref().map(|q| q.is_namespace_or_ns_constant()).unwrap_or(true);

        if !(has_known_ns && matches!(key, PropertyLookupKey::LocalName(_))) {
            verifier.add_verify_error(&id.location, WhackDiagnosticKind::DynamicOptionNotSupported, diagarg![]);
            return Ok(None);
        }

        let local_name = key.local_name().unwrap();

        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
        let lookup = PropertyLookup(&verifier.host).get_qname_in_ns_set_or_any_public_ns(&class.prototype(&verifier.host), &open_ns_set, qual, &local_name);

        if lookup.is_err() {
            match lookup.unwrap_err() {
                PropertyLookupError::AmbiguousReference(name) => {
                    verifier.add_verify_error(&id.location, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                    return Ok(None);
                },
                PropertyLookupError::Defer => {
                    return Err(DeferError(None));
                },
                _ => {
                    panic!();
                },
            }
        }

        let lookup = lookup.unwrap();
        let variable = lookup.and_then(|v| if v.is::<VariableSlot>() { Some(v) } else { None });
        if variable.is_none() {
            verifier.add_verify_error(&id.location, WhackDiagnosticKind::UnknownOptionForClass, diagarg![local_name]);
            return Ok(None);
        }
        Ok(Some(variable.unwrap()))
    }
}