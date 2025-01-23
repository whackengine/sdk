use crate::ns::*;

pub(crate) struct ExpSubverifier;

impl ExpSubverifier {
    // QualifiedIdentifier - returns (ns, local name)
    pub fn verify_qualified_identifier(verifier: &mut Subverifier, id: &QualifiedIdentifier) -> Result<Option<(Option<Entity>, PropertyLookupKey)>, DeferError> {
        let QualifiedIdentifier { qualifier, id, .. } = id;

        let mut failed = false;

        let mut result_qual: Option<Entity> = None;

        if let Some(qualifier) = qualifier {
            result_qual = verifier.imp_coerce_exp(qualifier, &verifier.host.namespace_type().defer()?)?;
            if result_qual.is_none() {
                failed = true;
            }
        }

        let mut result_key: Option<PropertyLookupKey> = None;

        match id {
            QualifiedIdentifierIdentifier::Id((id, _)) => {
                result_key = Some(PropertyLookupKey::LocalName(id.clone()));
            },
            QualifiedIdentifierIdentifier::Brackets(exp) => {
                let v = verifier.imp_coerce_exp(exp, &verifier.host.string_type().defer()?)?;
                if let Some(v) = v {
                    result_key = Some(PropertyLookupKey::Computed(v));
                } else {
                    failed = true;
                }
            },
        }

        if failed {
            Ok(None)
        } else {
            Ok(Some((result_qual, result_key.unwrap())))
        }
    }

    // QualifiedIdentifier
    pub fn verify_qualified_identifier_as_exp(verifier: &mut Subverifier, id: &QualifiedIdentifier, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        // Check for inline constants
        if let Some((name, cdata)) = Self::filter_inline_constant(verifier, id) {
            // Defer
            verifier.host.string_type().defer()?;
            verifier.host.non_null_primitive_types()?;

            return Ok(Self::eval_config_constant(verifier, &id.location, name, cdata));
        }

        let qn = Self::verify_qualified_identifier(verifier, id)?;
        if qn.is_none() {
            return Ok(None);
        }
        let (qual, key) = qn.unwrap();

        // Attribute
        if id.attribute {
            return Ok(Some(verifier.host.factory().create_dynamic_scope_reference_value(&verifier.scope(), qual, &key.computed_or_local_name(&verifier.host)?)));
        }

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

        // Mark local capture
        verifier.detect_local_capture(&r);

        // Post-processing
        verifier.reference_post_processing(r, context)
    }

    fn filter_inline_constant(verifier: &mut Subverifier, id: &QualifiedIdentifier) -> Option<(String, String)> {
        let QualifiedIdentifier { qualifier, id, .. } = id;

        if let Some(qualifier) = qualifier {
            // Detect any inline constant
            let inlinekqid = qualifier.to_identifier_name().map(|name| name.0);
            let inlinekln = if let QualifiedIdentifierIdentifier::Id((name, _)) = id { Some(name) } else { None };
            if let (Some(inlinekqid), Some(inlinekln)) = (inlinekqid, inlinekln) {
                let inlinekid = format!("{}::{}", inlinekqid, inlinekln);
                if let Some(cdata) = verifier.host.config_constants().get(&inlinekid) {
                    return Some((inlinekid, cdata));
                }
            }
        }

        None
    }

    pub fn eval_config_constant(verifier: &mut Subverifier, location: &Location, name: String, mut cdata: String) -> Option<Entity> {
        if let Some(v) = verifier.host.config_constants_result().get(&name) {
            if v.is::<InvalidationEntity>() {
                verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
                return None;
            }
            return Some(v);
        }

        verifier.host.config_constants_result().set(name.clone(), verifier.host.invalidation_entity());
        cdata = cdata.trim().to_owned();

        // Prevent creating an expression in general.

        if ["true", "false"].contains(&cdata.as_str()) {
            let boolean_type = verifier.host.boolean_type();
            if boolean_type.is::<UnresolvedEntity>() {
                verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
                return None;
            }
            let v = verifier.host.factory().create_boolean_constant(cdata == "true", &boolean_type);
            verifier.host.config_constants_result().set(name.clone(), v.clone());
            return Some(v);
        }

        if cdata == "Infinity" {
            let number_type = verifier.host.number_type();
            if number_type.is::<UnresolvedEntity>() {
                verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
                return None;
            }
            let v = verifier.host.factory().create_number_constant(Number::Number(f64::INFINITY), &number_type);
            verifier.host.config_constants_result().set(name.clone(), v.clone());
            return Some(v);
        }

        if cdata == "NaN" {
            let number_type = verifier.host.number_type();
            if number_type.is::<UnresolvedEntity>() {
                verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
                return None;
            }
            let v = verifier.host.factory().create_number_constant(Number::Number(f64::NAN), &number_type);
            verifier.host.config_constants_result().set(name.clone(), v.clone());
            return Some(v);
        }

        let cu = CompilationUnit::new(Some(name.clone()), cdata);
        cu.set_compiler_options(location.compilation_unit().compiler_options());
        location.compilation_unit().add_nested_compilation_unit(cu.clone());

        // Build an expression for the constant,
        // which must be a compile-time constant.
        let exp = ParserFacade(&cu, ParserOptions::default()).parse_expression();
        if cu.invalidated() {
            verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
            return None;
        }
        let kscope = verifier.scope();
        verifier.set_scope(&verifier.host.const_eval_scope());
        let cval = verifier.verify_expression(&exp, &default());
        verifier.set_scope(&kscope);
        let Ok(cval) = cval else {
            verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
            return None;
        };
        if let Some(cval) = cval.as_ref() {
            if !cval.is::<Constant>() {
                verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
                return None;
            }
            verifier.host.config_constants_result().set(name.clone(), cval.clone());
        } else {
            verifier.add_verify_error(location, WhackDiagnosticKind::CouldNotExpandInlineConstant, diagarg![]);
            return None;
        }
        cval
    }

    pub fn verify_null_literal(verifier: &mut Subverifier, literal: &NullLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        if let Some(t) = context.context_type.as_ref() {
            if t.includes_null(&verifier.host)? {
                return Ok(Some(verifier.host.factory().create_null_constant(t)));
            } else {
                verifier.add_verify_error(&literal.location, WhackDiagnosticKind::NullNotExpectedHere, diagarg![]);
                return Ok(None);
            }
        }
        Ok(Some(verifier.host.factory().create_null_constant(&verifier.host.any_type())))
    }

    pub fn verify_boolean_literal(verifier: &mut Subverifier, literal: &BooleanLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if [verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.boolean_type().defer()?].contains(&t_esc) {
                return Ok(Some(verifier.host.factory().create_boolean_constant(literal.value, &t)));
            }
        }
        Ok(Some(verifier.host.factory().create_boolean_constant(literal.value, &verifier.host.boolean_type().defer()?)))
    }

    pub fn verify_numeric_literal(verifier: &mut Subverifier, literal: &NumericLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if verifier.host.numeric_types()?.contains(&t_esc) {
                let n = Self::parse_number_as_data_type(&verifier.host, literal, &t_esc, context);
                if n.is_err() {
                    verifier.add_verify_error(&literal.location, WhackDiagnosticKind::CouldNotParseNumber, diagarg![t_esc]);
                    return Ok(None);
                }
                return Ok(Some(verifier.host.factory().create_number_constant(n.unwrap(), t)));
            }
        }
        let t = if literal.suffix == NumberSuffix::F { verifier.host.float_type() } else { verifier.host.number_type() }.defer()?;
        let n = Self::parse_number_as_data_type(&verifier.host, literal, &t, context);
        if n.is_err() {
            verifier.add_verify_error(&literal.location, WhackDiagnosticKind::CouldNotParseNumber, diagarg![t]);
            return Ok(None);
        }
        Ok(Some(verifier.host.factory().create_number_constant(n.unwrap(), &t)))
    }

    pub fn parse_number_as_data_type(host: &Database, literal: &NumericLiteral, data_type: &Entity, context: &VerifierExpressionContext) -> Result<Number, ParserError> {
        if data_type == &host.number_type() {
            Ok(Number::Number(literal.parse_double(context.preceded_by_negative)?))
        } else if data_type == &host.float_type() {
            Ok(Number::Float(literal.parse_float(context.preceded_by_negative)?))
        } else if data_type == &host.int_type() {
            Ok(Number::Int(literal.parse_int(context.preceded_by_negative)?))
        } else {
            assert!(data_type == &host.uint_type());
            Ok(Number::Uint(literal.parse_uint()?))
        }
    }

    pub fn verify_string_literal(verifier: &mut Subverifier, literal: &StringLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if t_esc.is::<EnumType>() {
                let slot = t_esc.enum_member_slot_mapping().get(&literal.value);
                if let Some(slot) = slot {
                    let k = verifier.host.factory().create_static_reference_value(&t_esc, &slot)?;
                    return Ok(ConversionMethods(&verifier.host).implicit(&k, &t, false)?);
                } else {
                    verifier.add_verify_error(&literal.location, WhackDiagnosticKind::NoMatchingEnumMember, diagarg![literal.value.clone(), t_esc]);
                    return Ok(None);
                }
            }
        }
        return Ok(Some(verifier.host.factory().create_string_constant(literal.value.clone(), &verifier.host.string_type().defer()?)));
    }

    pub fn verify_this_literal(verifier: &mut Subverifier, literal: &ThisLiteral) -> Result<Option<Entity>, DeferError> {
        let activation = verifier.scope().search_activation();
        if activation.is_some() && activation.as_ref().unwrap().this().is_some() {
            Ok(activation.clone().unwrap().this())
        } else {
            verifier.add_verify_error(&literal.location, WhackDiagnosticKind::UnexpectedThis, diagarg![]);
            Ok(None)
        }
    }

    pub fn verify_reg_exp_literal(verifier: &mut Subverifier, _literal: &RegExpLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if [verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.reg_exp_type().defer()?].contains(&t_esc) {
                return Ok(Some(verifier.host.factory().create_value(&t)));
            }
        }
        Ok(Some(verifier.host.factory().create_value(&verifier.host.reg_exp_type().defer()?)))
    }

    pub fn verify_xml_exp(verifier: &mut Subverifier, exp: &XmlExpression, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        Self::verify_xml_elem(verifier, &exp.element)?;
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if [verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.xml_type().defer()?].contains(&t_esc) {
                return Ok(Some(verifier.host.factory().create_value(&t)));
            }
        }
        Ok(Some(verifier.host.factory().create_value(&verifier.host.xml_type().defer()?)))
    }

    pub fn verify_xml_list_exp(verifier: &mut Subverifier, exp: &XmlListExpression, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        for content in exp.content.iter() {
            Self::verify_xml_content(verifier, content)?;
        }
        if let Some(t) = context.context_type.as_ref() {
            let t_esc = t.escape_of_nullable_or_non_nullable();
            if [verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.xml_list_type().defer()?].contains(&t_esc) {
                return Ok(Some(verifier.host.factory().create_value(&t)));
            }
        }
        Ok(Some(verifier.host.factory().create_value(&verifier.host.xml_list_type().defer()?)))
    }

    pub fn verify_xml_elem(verifier: &mut Subverifier, elem: &XmlElement) -> Result<(), DeferError> {
        if let XmlTagName::Expression(exp) = &elem.name {
            verifier.verify_expression(exp, &VerifierExpressionContext { ..default() })?;
        }
        for attr in &elem.attributes {
            if let XmlAttributeValue::Expression(exp) = &attr.value {
                verifier.verify_expression(exp, &VerifierExpressionContext { ..default() })?;
            }
        }
        if let Some(exp) = &elem.attribute_expression {
            verifier.verify_expression(exp, &VerifierExpressionContext { ..default() })?;
        }
        if let Some(content_list) = &elem.content {
            for content in content_list {
                Self::verify_xml_content(verifier, content)?;
            }
        }
        if let Some(XmlTagName::Expression(exp)) = &elem.closing_name {
            verifier.verify_expression(exp, &VerifierExpressionContext { ..default() })?;
        }
        Ok(())
    }

    pub fn verify_xml_content(verifier: &mut Subverifier, content: &Rc<XmlContent>) -> Result<(), DeferError> {
        match content.as_ref() {
            XmlContent::Element(elem) => {
                Self::verify_xml_elem(verifier, elem)?;
                Ok(())
            },
            XmlContent::Expression(exp) => {
                verifier.verify_expression(exp, &VerifierExpressionContext { ..default() })?;
                Ok(())
            },
            _ => Ok(()),
        }
    }

    pub fn verify_new_exp(verifier: &mut Subverifier, exp: &NewExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&exp.base, &default())? else {
            if let Some(arguments) = &exp.arguments {
                for arg in arguments.iter() {
                    verifier.verify_expression(arg, &default())?;
                }
            }
            return Ok(None);
        };

        if let Some(t) = base.as_type() {
            if !(t.is_class_type_possibly_after_sub() && !t.is_static() && !t.is_abstract()) {
                verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::UnexpectedNewBase, diagarg![]);

                if let Some(arguments) = &exp.arguments {
                    for arg in arguments.iter() {
                        verifier.verify_expression(arg, &default())?;
                    }
                }

                return Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())));
            }

            // In AS3, the constructor is not inherited unlike in other languages.
            let ctor = t.constructor_method(&verifier.host);

            if let Some(ctor) = ctor {
                let sig = ctor.signature(&verifier.host).defer()?;
                match ArgumentsSubverifier::verify(verifier, exp.arguments.as_ref().unwrap_or(&vec![]), &sig) {
                    Ok(_) => {},
                    Err(VerifierArgumentsError::Defer) => {
                        return Err(DeferError(None));
                    },
                    Err(VerifierArgumentsError::Expected(n)) => {
                        verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArguments, diagarg![n.to_string()]);
                    },
                    Err(VerifierArgumentsError::ExpectedNoMoreThan(n)) => {
                        verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg![n.to_string()]);
                    },
                }
            } else {
                if let Some(arguments) = &exp.arguments {
                    if !arguments.is_empty() {
                        verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg!["0".to_string()]);
                    }
                    for arg in arguments.iter() {
                        verifier.verify_expression(arg, &default())?;
                    }
                }
            }

            return Ok(Some(verifier.host.factory().create_value(&t)));
        }

        let base_st = base.static_type(&verifier.host);
        let base_st_esc = base_st.escape_of_non_nullable();

        if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.class_type().defer()?, verifier.host.jsval_type().defer()?].contains(&base_st_esc) {
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::UnexpectedNewBase, diagarg![]);
        }

        if let Some(arguments) = &exp.arguments {
            for arg in arguments.iter() {
                verifier.verify_expression(arg, &default())?;
            }
        }

        return Ok(Some(verifier.host.factory().create_value(&if base_st_esc == verifier.host.jsval_type() { base_st_esc } else { verifier.host.any_type() })));
    }

    pub fn verify_member_exp(verifier: &mut Subverifier, exp: &Rc<Expression>, member_exp: &MemberExpression, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        // Shadowing package names
        if let Some(r) = Self::verify_member_exp_pre_package_names(verifier, exp, member_exp)? {
            return Ok(Some(r));
        }

        let id = &member_exp.identifier;

        let Some(base) = verifier.verify_expression(&member_exp.base, &default())? else {
            Self::verify_qualified_identifier(verifier, id)?;
            return Ok(None);
        };

        let qn = Self::verify_qualified_identifier(verifier, id)?;
        if qn.is_none() {
            return Ok(None);
        }
        let (qual, key) = qn.unwrap();

        // Attribute
        if id.attribute {
            return Ok(Some(verifier.host.factory().create_dynamic_reference_value(&base, qual, &key.computed_or_local_name(&verifier.host)?)));
        }

        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
        let r = PropertyLookup(&verifier.host).lookup_in_object(&base, &open_ns_set, qual, &key, context.followed_by_call);
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
            verifier.add_verify_error(&id.location, WhackDiagnosticKind::UndefinedPropertyWithStaticType, diagarg![key.local_name().unwrap(), base.static_type(&verifier.host)]);
            return Ok(None);
        }
        let r = r.unwrap();

        // No need to mark local capture for the property operator.
        // verifier.detect_local_capture(&r);

        // Post-processing
        verifier.reference_post_processing(r, context)
    }

    fn verify_member_exp_pre_package_names(verifier: &mut Subverifier, exp: &Rc<Expression>, member_exp: &MemberExpression) -> Result<Option<Entity>, DeferError> {
        let Some(dot_seq) = Self::dot_delimited_id_sequence(exp) else {
            return Ok(None);
        };
        let mut scope = Some(verifier.scope());
        while let Some(scope1) = scope {
            let open_ns_set = scope1.concat_open_ns_set_of_scope_chain();
            let mut r: Option<Entity> = None;
            if scope1.is::<PackageScope>() {
                let p = scope1.package();
                if &dot_seq[0..(dot_seq.len() - 1)] == &p.fully_qualified_name_list() {
                    match PropertyLookup(&verifier.host).lookup_in_object(&p, &open_ns_set, None, &PropertyLookupKey::LocalName(dot_seq.last().unwrap().clone()), false) {
                        Ok(Some(r1)) => {
                            if r.is_some() && !r.as_ref().unwrap().fixture_reference_value_equals(&r1) {
                                verifier.add_verify_error(&member_exp.identifier.location, WhackDiagnosticKind::AmbiguousReference, diagarg![dot_seq.last().unwrap().clone()]);
                                return Ok(None);
                            }
                            r = Some(r1);
                        },
                        Ok(None) => {},
                        Err(PropertyLookupError::AmbiguousReference(name)) => {
                            verifier.add_verify_error(&member_exp.identifier.location, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                            return Ok(Some(verifier.host.invalidation_entity()));
                        },
                        Err(PropertyLookupError::Defer) => {
                            return Err(DeferError(None));
                        },
                        Err(_) => {
                            panic!();
                        },
                    }
                }
            }
            for import in scope1.import_list().iter() {
                if let Some(r1) = Self::import_shadowing_package_name(verifier, &open_ns_set, &dot_seq, &import, &member_exp.identifier.location)? {
                    if r.is_some() && !r.as_ref().unwrap().fixture_reference_value_equals(&r1) {
                        verifier.add_verify_error(&member_exp.identifier.location, WhackDiagnosticKind::AmbiguousReference, diagarg![dot_seq.last().unwrap().clone()]);
                        return Ok(None);
                    }
                    r = Some(r1);
                }
            }
            if let Some(r) = r {
                return Ok(Some(r));
            }
            scope = scope1.parent();
        }
        Ok(None)
    }

    fn import_shadowing_package_name(verifier: &mut Subverifier, open_ns_set: &SharedArray<Entity>, dot_seq: &Vec<String>, import: &Entity, location: &Location) -> Result<Option<Entity>, DeferError> {
        if import.is::<PackageWildcardImport>() {
            if &dot_seq[0..(dot_seq.len() - 1)] != &import.package().fully_qualified_name_list() {
                return Ok(None);
            }
            match PropertyLookup(&verifier.host).lookup_in_object(&import.package(), &open_ns_set, None, &PropertyLookupKey::LocalName(dot_seq.last().unwrap().clone()), false) {
                Ok(Some(r)) => {
                    Unused(&verifier.host).mark_used(import);
                    return Ok(Some(r));
                },
                Ok(None) => {
                    return Ok(None);
                },
                Err(PropertyLookupError::AmbiguousReference(name)) => {
                    verifier.add_verify_error(&location, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                    return Ok(Some(verifier.host.invalidation_entity()));
                },
                Err(PropertyLookupError::Defer) => {
                    return Err(DeferError(None));
                },
                Err(_) => {
                    panic!();
                },
            }
        } else if import.is::<PackageRecursiveImport>() {
            if &dot_seq[0..(dot_seq.len() - 1)] != &import.package().fully_qualified_name_list() {
                return Ok(None);
            }
            match PropertyLookup(&verifier.host).lookup_in_package_recursive(&import.package(), &open_ns_set, None, &PropertyLookupKey::LocalName(dot_seq.last().unwrap().clone())) {
                Ok(Some(r)) => {
                    Unused(&verifier.host).mark_used(import);
                    return Ok(Some(r));
                },
                Ok(None) => {
                    return Ok(None);
                },
                Err(PropertyLookupError::AmbiguousReference(name)) => {
                    verifier.add_verify_error(&location, WhackDiagnosticKind::AmbiguousReference, diagarg![name.clone()]);
                    return Ok(Some(verifier.host.invalidation_entity()));
                },
                Err(PropertyLookupError::Defer) => {
                    return Err(DeferError(None));
                },
                Err(_) => {
                    panic!();
                },
            }
        } else {
            assert!(import.is::<PackagePropertyImport>());
            let prop = import.property();
            prop.defer()?;
            if prop.is::<InvalidationEntity>() {
                return Ok(None);
            }
            if &dot_seq[0..(dot_seq.len() - 1)] != &prop.parent().unwrap().fully_qualified_name_list()
            || dot_seq.last().unwrap() != &prop.name().local_name()
            {
                return Ok(None);
            }
            Unused(&verifier.host).mark_used(import);
            Ok(Some(prop.resolve_alias().wrap_property_reference(&verifier.host)?))
        }
    }

    fn dot_delimited_id_sequence(exp: &Rc<Expression>) -> Option<Vec<String>> {
        match exp.as_ref() {
            Expression::QualifiedIdentifier(id) => {
                id.to_identifier_name().map(|name| vec![name.0.clone()])
            },
            Expression::Member(m) => {
                let mut seq = Self::dot_delimited_id_sequence(&m.base)?;
                seq.push(m.identifier.to_identifier_name()?.0.clone());
                Some(seq)
            },
            _ => None,
        }
    }

    pub fn verify_computed_member_exp(verifier: &mut Subverifier, member_exp: &ComputedMemberExpression, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&member_exp.base, &default())? else {
            verifier.verify_expression(&member_exp.key, &default())?;
            return Ok(None);
        };

        let Some(key) = verifier.verify_expression(&member_exp.key, &default())? else {
            return Ok(None);
        };

        let open_ns_set = verifier.scope().concat_open_ns_set_of_scope_chain();
        let r = PropertyLookup(&verifier.host).lookup_in_object(&base, &open_ns_set, None, &PropertyLookupKey::Computed(key.clone()), context.followed_by_call);
        if r.is_err() {
            match r.unwrap_err() {
                PropertyLookupError::AmbiguousReference(_) => {
                    panic!();
                },
                PropertyLookupError::Defer => {
                    return Err(DeferError(None));
                },
                PropertyLookupError::VoidBase => {
                    verifier.add_verify_error(&member_exp.key.location(), WhackDiagnosticKind::AccessOfVoid, diagarg![]);
                    return Ok(None);
                },
                PropertyLookupError::NullableObject { .. } => {
                    verifier.add_verify_error(&member_exp.key.location(), WhackDiagnosticKind::AccessOfNullable, diagarg![]);
                    return Ok(None);
                },
            }
        }
        let r = r.unwrap();
        if r.is_none() {
            panic!();
        }
        let r = r.unwrap();

        // No need to mark local capture for the property operator.
        // verifier.detect_local_capture(&r);

        // Post-processing
        verifier.reference_post_processing(r, context)
    }

    pub fn verify_descendants_exp(verifier: &mut Subverifier, desc_exp: &DescendantsExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&desc_exp.base, &default())? else {
            Self::verify_qualified_identifier(verifier, &desc_exp.identifier)?;
            return Ok(None);
        };

        Self::verify_qualified_identifier(verifier, &desc_exp.identifier)?;

        let base_st = base.static_type(&verifier.host);
        let base_st_esc = base_st.escape_of_non_nullable();

        if ![verifier.host.any_type(),
            verifier.host.object_type().defer()?,
            verifier.host.xml_type().defer()?,
            verifier.host.xml_list_type().defer()?].contains(&base_st_esc) {
            verifier.add_verify_error(&desc_exp.identifier.location, WhackDiagnosticKind::InapplicableDescendants, diagarg![base_st]);
            return Ok(None);
        }

        if [verifier.host.any_type(), verifier.host.object_type()].contains(&base_st_esc) {
            return Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())));
        }

        Ok(Some(verifier.host.factory().create_value(&verifier.host.xml_list_type())))
    }

    pub fn verify_filter_exp(verifier: &mut Subverifier, filter_exp: &FilterExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&filter_exp.base, &default())? else {
            let scope = verifier.host.factory().create_filter_scope(&verifier.host.invalidation_entity());
            verifier.inherit_and_enter_scope(&scope);
            verifier.verify_expression(&filter_exp.test, &default())?;
            verifier.exit_scope();

            return Ok(None);
        };

        let scope = verifier.host.factory().create_filter_scope(&base);
        verifier.inherit_and_enter_scope(&scope);
        verifier.verify_expression(&filter_exp.test, &default())?;
        verifier.exit_scope();

        let base_st = base.static_type(&verifier.host);
        let base_st_esc = base_st.escape_of_non_nullable();

        if ![verifier.host.any_type(),
            verifier.host.object_type().defer()?,
            verifier.host.xml_type().defer()?,
            verifier.host.xml_list_type().defer()?].contains(&base_st_esc) {
            verifier.add_verify_error(&filter_exp.test.location(), WhackDiagnosticKind::InapplicableFilter, diagarg![base_st]);
            return Ok(None);
        }

        if [verifier.host.any_type(), verifier.host.object_type()].contains(&base_st_esc) {
            return Ok(Some(verifier.host.factory().create_filter_value(&scope, &verifier.host.any_type())));
        }

        Ok(Some(verifier.host.factory().create_filter_value(&scope, &verifier.host.xml_list_type())))
    }

    pub fn verify_super_exp(verifier: &mut Subverifier, super_exp: &SuperExpression) -> Result<Option<Entity>, DeferError> {
        let Some(act) = verifier.scope().search_activation() else {
            if let Some(object) = super_exp.object.as_ref() {
                for obj in object {
                    verifier.verify_expression(obj, &default())?;
                }
            }
            verifier.add_verify_error(&super_exp.location, WhackDiagnosticKind::ASuperExpCanBeUsedOnlyIn, diagarg![]);
            return Ok(None);
        };

        let Some(this) = act.this() else {
            if let Some(object) = super_exp.object.as_ref() {
                for obj in object {
                    verifier.verify_expression(obj, &default())?;
                }
            }
            verifier.add_verify_error(&super_exp.location, WhackDiagnosticKind::ASuperExpCanBeUsedOnlyIn, diagarg![]);
            return Ok(None);
        };

        // In the future, provided interface methods could be supported, so
        // come with a pre-check.
        let this_st = this.static_type(&verifier.host);
        if this_st.is_interface_type_possibly_after_sub() {
            if let Some(object) = super_exp.object.as_ref() {
                for obj in object {
                    verifier.verify_expression(obj, &default())?;
                }
            }
            verifier.add_verify_error(&super_exp.location, WhackDiagnosticKind::ASuperExpCanBeUsedOnlyIn, diagarg![]);
            return Ok(None);
        }


        let Some(limit) = this_st.extends_class(&verifier.host) else {
            if let Some(object) = super_exp.object.as_ref() {
                for obj in object {
                    verifier.verify_expression(obj, &default())?;
                }
            }
            verifier.add_verify_error(&super_exp.location, WhackDiagnosticKind::ASuperExpCanOnlyBeUsedInSubclasses, diagarg![]);
            return Ok(None);
        };

        limit.defer()?;

        if let Some(object) = super_exp.object.as_ref() {
            if !object.is_empty() {
                for obj in &object[..(object.len() - 1)] {
                    verifier.verify_expression(obj, &default())?;
                }
                verifier.imp_coerce_exp(object.last().unwrap(), &limit)?;
            }
        }

        Ok(Some(verifier.host.factory().create_value(&limit)))
    }

    pub fn verify_call_exp(verifier: &mut Subverifier, exp: &CallExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&exp.base, &VerifierExpressionContext {
            followed_by_call: true,
            ..default()
        })? else {
            for arg in &exp.arguments {
                verifier.verify_expression(arg, &default())?;
            }
            return Ok(None);
        };

        // Type cast or new Array
        if let Some(base_type) = base.as_type() {
            let array_type = verifier.host.array_type().defer()?;
            let date_type = verifier.host.date_type().defer()?;
            // new Array
            if base_type == array_type || base_type.type_after_sub_has_origin(&array_type) {
                for arg in &exp.arguments {
                    verifier.verify_expression(arg, &default())?;
                }
                verifier.add_warning(&exp.base.location(), WhackDiagnosticKind::CallOnArrayType, diagarg![]);
                return Ok(Some(verifier.host.factory().create_value(&array_type)));
            // Date(x)
            } else if base_type == date_type {
                let string_type = verifier.host.string_type().defer()?;
                for arg in &exp.arguments {
                    verifier.verify_expression(arg, &default())?;
                }
                verifier.add_warning(&exp.base.location(), WhackDiagnosticKind::CallOnDateType, diagarg![]);
                return Ok(Some(verifier.host.factory().create_value(&string_type)));
            // Type cast
            } else {
                let mut first = true;
                for arg in &exp.arguments {
                    if first {
                        verifier.verify_expression(arg, &VerifierExpressionContext {
                            context_type: Some(base_type.clone()),
                            ..default()
                        })?;
                    } else {
                        verifier.verify_expression(arg, &default())?;
                    }
                    first = false;
                }
                if exp.arguments.len() < 1 {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArguments, diagarg!["1".to_string()]);
                } else if exp.arguments.len() > 1 {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg!["1".to_string()]);
                }
                return Ok(Some(verifier.host.factory().create_value(&base_type)));
            }
        }

        if base.is::<FixtureReferenceValue>() && base.property().is::<MethodSlot>() {
            let sig = base.property().signature(&verifier.host).defer()?;
            match ArgumentsSubverifier::verify(verifier, &exp.arguments, &sig) {
                Ok(_) => {},
                Err(VerifierArgumentsError::Defer) => {
                    return Err(DeferError(None));
                },
                Err(VerifierArgumentsError::Expected(n)) => {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArguments, diagarg![n.to_string()]);
                },
                Err(VerifierArgumentsError::ExpectedNoMoreThan(n)) => {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg![n.to_string()]);
                },
            }
            return Ok(Some(verifier.host.factory().create_value(&sig.result_type())));
        }

        let base_st = base.static_type(&verifier.host);
        let base_st_esc = base_st.escape_of_non_nullable();
        let class_type = verifier.host.class_type().defer()?;

        // Check call to a Class typed object.
        if base_st_esc == class_type {
            for arg in &exp.arguments {
                verifier.verify_expression(arg, &default())?;
            }
            return Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())));
        }

        if base_st_esc.is::<FunctionType>() {
            let sig = base_st_esc;
            match ArgumentsSubverifier::verify(verifier, &exp.arguments, &sig) {
                Ok(_) => {},
                Err(VerifierArgumentsError::Defer) => {
                    return Err(DeferError(None));
                },
                Err(VerifierArgumentsError::Expected(n)) => {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArguments, diagarg![n.to_string()]);
                },
                Err(VerifierArgumentsError::ExpectedNoMoreThan(n)) => {
                    verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg![n.to_string()]);
                },
            }
            return Ok(Some(verifier.host.factory().create_value(&sig.result_type())));
        }

        for arg in &exp.arguments {
            verifier.verify_expression(arg, &default())?;
        }

        if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.function_type().defer()?, verifier.host.jsval_type().defer()?].contains(&base_st_esc) {
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::CallOnNonFunction, diagarg![]);
            return Ok(None);
        }

        Ok(Some(verifier.host.factory().create_value(&if base_st_esc == verifier.host.jsval_type() { base_st_esc } else { verifier.host.any_type() })))
    }

    pub fn verify_apply_types_exp(verifier: &mut Subverifier, exp: &ApplyTypeExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_expression(&exp.base, &VerifierExpressionContext {
            followed_by_type_arguments: true,
            ..default()
        })? else {
            for arg in &exp.arguments {
                verifier.verify_type_expression(arg)?;
            }
            return Ok(None);
        };

        // Ensure base is a type
        let Ok(base) = base.expect_type() else {
            for arg in &exp.arguments {
                verifier.verify_type_expression(arg)?;
            }
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::EntityIsNotAType, diagarg![]);
            return Ok(None);
        };

        // Ensure type is parameterized
        if !((base.is::<ClassType>() || base.is::<InterfaceType>()) && base.type_params().is_some()) {
            for arg in &exp.arguments {
                verifier.verify_type_expression(arg)?;
            }
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::NonParameterizedType, diagarg![]);
            return Ok(None);
        }

        let mut resolvee_args: SharedArray<Entity> = shared_array![];
        let mut valid = true;

        for arg in &exp.arguments {
            if let Some(t) = verifier.verify_type_expression(arg)? {
                resolvee_args.push(t);
            } else {
                resolvee_args.push(verifier.host.invalidation_entity());
                valid = false;
            }
        }

        let type_params = base.type_params().unwrap();

        if resolvee_args.length() < type_params.length() {
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArguments, diagarg![type_params.length().to_string()]);
            return Ok(None);
        } else if resolvee_args.length() > type_params.length() {
            verifier.add_verify_error(&exp.base.location(), WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg![type_params.length().to_string()]);
            return Ok(None);
        }

        if !valid {
            return Ok(None);
        }

        Ok(Some(verifier.host.factory().create_type_after_substitution(&base, &resolvee_args).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_unary_exp(verifier: &mut Subverifier, exp: &UnaryExpression) -> Result<Option<Entity>, DeferError> {
        if exp.operator == Operator::Await {
            let Some(val) = verifier.verify_expression(&exp.expression, &default())? else {
                return Ok(None);
            };

            let val_st = val.static_type(&verifier.host);
            let Some(result_type) = val_st.escape_of_non_nullable().promise_result_type(&verifier.host)? else {
                verifier.add_verify_error(&exp.location, WhackDiagnosticKind::AwaitOperandMustBeAPromise, diagarg![]);
                return Ok(None);
            };

            return Ok(Some(verifier.host.factory().create_value(&result_type)));
        }

        let update_ops = [Operator::PreIncrement, Operator::PreDecrement, Operator::PostIncrement, Operator::PostDecrement];
        let rw_mode = if exp.operator == Operator::Delete {
            VerifyMode::Delete
        } else if update_ops.contains(&exp.operator) {
            VerifyMode::Write
        } else {
            VerifyMode::Read
        };
        let Some(val) = verifier.verify_expression(&exp.expression, &VerifierExpressionContext {
            preceded_by_negative: exp.operator == Operator::Negative,
            mode: rw_mode,
            ..default()
        })? else {
            return Ok(None);
        };

        let val_st = val.static_type(&verifier.host);

        match exp.operator {
            Operator::PreIncrement |
            Operator::PreDecrement |
            Operator::PostIncrement |
            Operator::PostDecrement => {
                let val_st_esc = val_st.escape_of_non_nullable();
                if !([verifier.host.any_type(), verifier.host.jsval_type().defer()?].contains(&val_st_esc) || verifier.host.numeric_types()?.contains(&val_st)) {
                    verifier.add_verify_error(&exp.expression.location(), WhackDiagnosticKind::OperandMustBeNumber, diagarg![]);
                } else if val.write_only(&verifier.host) {
                    verifier.add_verify_error(&exp.expression.location(), WhackDiagnosticKind::EntityIsWriteOnly, diagarg![]);
                }
                Ok(Some(verifier.host.factory().create_value(&val_st)))
            },
            Operator::NonNull => {
                if val_st.includes_undefined(&verifier.host)? || val_st.includes_null(&verifier.host)? {
                    let non_null_t = verifier.host.factory().create_non_nullable_type(&val_st);
                    Ok(Some(verifier.host.factory().create_value(&non_null_t)))
                } else {
                    verifier.add_warning(&exp.expression.location(), WhackDiagnosticKind::ReferenceIsAlreadyNonNullable, diagarg![]);
                    Ok(Some(verifier.host.factory().create_value(&val_st)))
                }
            },
            Operator::Delete => {
                Ok(Some(verifier.host.factory().create_value(&verifier.host.boolean_type().defer()?)))
            },
            Operator::Void => {
                Ok(Some(verifier.host.factory().create_undefined_constant(&verifier.host.any_type())))
            },
            Operator::Typeof => {
                Ok(Some(verifier.host.factory().create_value(&verifier.host.string_type().defer()?)))
            },
            Operator::Yield => {
                verifier.add_verify_error(&exp.location, WhackDiagnosticKind::YieldIsNotSupported, diagarg![]);
                Ok(None)
            },
            Operator::Positive => {
                let val_st_esc = val_st.escape_of_non_nullable();
                if !([verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&val_st_esc) || verifier.host.numeric_types()?.contains(&val_st)) {
                    verifier.add_verify_error(&exp.expression.location(), WhackDiagnosticKind::OperandMustBeNumber, diagarg![]);
                    return Ok(None);
                }
                if val.is::<NumberConstant>() {
                    return Ok(Some(val.clone()));
                }
                Ok(Some(verifier.host.factory().create_value(&val_st)))
            },
            Operator::Negative => {
                let val_st_esc = val_st.escape_of_non_nullable();
                if !([verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&val_st_esc) || verifier.host.numeric_types()?.contains(&val_st)) {
                    verifier.add_verify_error(&exp.expression.location(), WhackDiagnosticKind::OperandMustBeNumber, diagarg![]);
                    return Ok(None);
                }
                if val.is::<NumberConstant>() {
                    // Numeric literal has already been negated.
                    if matches!(exp.expression.as_ref(), Expression::NumericLiteral(_)) {
                        return Ok(Some(val.clone()));
                    }
                    return Ok(Some(verifier.host.factory().create_number_constant(-val.number_value(), &val_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&val_st)))
            },
            Operator::BitwiseNot => {
                let val_st_esc = val_st.escape_of_non_nullable();
                if !([verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&val_st_esc) || verifier.host.numeric_types()?.contains(&val_st)) {
                    verifier.add_verify_error(&exp.expression.location(), WhackDiagnosticKind::OperandMustBeNumber, diagarg![]);
                    return Ok(None);
                }
                if val.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(val.number_value().bitwise_not(), &val_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&val_st)))
            },
            Operator::LogicalNot => {
                if val.is::<Constant>() {
                    if val.is::<BooleanConstant>() {
                        return Ok(Some(verifier.host.factory().create_boolean_constant(!val.boolean_value(), &verifier.host.boolean_type().defer()?)));
                    } else if val.is::<NumberConstant>() {
                        let mv = val.number_value();
                        return Ok(Some(verifier.host.factory().create_boolean_constant(mv.is_zero() || mv.is_nan(), &verifier.host.boolean_type().defer()?)));
                    } else if val.is::<StringConstant>() {
                        return Ok(Some(verifier.host.factory().create_boolean_constant(val.string_value().is_empty(), &verifier.host.boolean_type().defer()?)));
                    } else if val.is::<UndefinedConstant>() {
                        return Ok(Some(verifier.host.factory().create_boolean_constant(true, &verifier.host.boolean_type().defer()?)));
                    } else if val.is::<NullConstant>() {
                        return Ok(Some(verifier.host.factory().create_boolean_constant(true, &verifier.host.boolean_type().defer()?)));
                    }
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.boolean_type().defer()?)))
            },
            _ => {
                panic!();
            },
        }
    }

    pub fn verify_opt_chaining_exp(verifier: &mut Subverifier, exp: &OptionalChainingExpression) -> Result<Option<Entity>, DeferError> {
        let placeholder = exp.expression.search_optional_chaining_placeholder().unwrap();

        let Some(base) = verifier.verify_expression(&exp.base, &default())? else {
            verifier.host.node_mapping().set(&placeholder, None);
            verifier.verify_expression(&exp.expression, &default())?;
            return Ok(None);
        };

        let base_st = base.static_type(&verifier.host);
        let base_st_esc = base_st.escape_of_nullable_or_non_nullable();
        let base_st_esc_is_opt = base_st_esc.includes_null(&verifier.host)? || base_st_esc.includes_undefined(&verifier.host)?;

        let non_null_t = if base_st_esc_is_opt { verifier.host.factory().create_non_nullable_type(&base_st_esc) } else { base_st_esc };

        // Assign placeholder's value
        verifier.host.node_mapping().set(&placeholder, Some(verifier.host.factory().create_value(&non_null_t)));

        // Verify subexpressions
        let Some(expval) = verifier.verify_expression(&exp.expression, &default())? else {
            // Report warning
            if !base_st_esc_is_opt {
                verifier.add_warning(&exp.base.location(), WhackDiagnosticKind::ReferenceIsAlreadyNonNullable, diagarg![]);
            }

            return Ok(None);
        };

        let expval_st = expval.static_type(&verifier.host);
        let nullable_result_type = if expval_st == verifier.host.object_type().defer()? || expval_st == verifier.host.jsval_type().defer()? {
            expval_st.clone()
        } else {
            verifier.host.factory().create_nullable_type(&expval_st)
        };

        // Report warning
        if !base_st_esc_is_opt {
            verifier.add_warning(&exp.base.location(), WhackDiagnosticKind::ReferenceIsAlreadyNonNullable, diagarg![]);
        }

        Ok(Some(verifier.host.factory().create_value(&nullable_result_type)))
    }

    pub fn verify_binary_exp(verifier: &mut Subverifier, exp: &BinaryExpression) -> Result<Option<Entity>, DeferError> {
        let Some(left) = verifier.verify_expression(&exp.left, &default())? else {
            verifier.verify_expression(&exp.right, &default())?;
            return Ok(None);
        };

        let left_st = left.static_type(&verifier.host);
        let left_st_esc = left_st.escape_of_non_nullable();

        match exp.operator {
            Operator::Add => {
                let Some(right) = verifier.verify_expression(&exp.right, &VerifierExpressionContext {
                    context_type: Some(left_st.clone()),
                    ..default()
                })? else {
                    return Ok(None);
                };
                let object_type = verifier.host.object_type().defer()?;
                let numeric_types = verifier.host.numeric_types()?;

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() + right.number_value(), &left_st)));
                }
                if left.is::<StringConstant>() && right.is::<StringConstant>() {
                    return Ok(Some(verifier.host.factory().create_string_constant(left.string_value() + &right.string_value(), &left_st)));
                }
                if numeric_types.contains(&left_st) || left_st.escape_of_non_nullable() == object_type {
                    return Ok(Some(verifier.host.factory().create_value(&left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())))
            },
            Operator::Subtract => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() - right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::Multiply => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() * right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::Divide => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() / right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::Remainder => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() % right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::Power => {
                let Some(_) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::BitwiseAnd => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() & right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::BitwiseXor => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() ^  right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::BitwiseOr => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() | right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::ShiftLeft => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() << right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::ShiftRight => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value() >> right.number_value(), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::ShiftRightUnsigned => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };
                if ![verifier.host.any_type(), verifier.host.object_type().defer()?, verifier.host.jsval_type().defer()?].contains(&left_st_esc)
                && !verifier.host.numeric_types()?.contains(&left_st_esc)
                {
                    verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st]);
                    return Ok(None);
                }
                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_number_constant(left.number_value().shift_right_unsigned(&right.number_value()), &left_st)));
                }
                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            Operator::Equals |
            Operator::StrictEquals => {
                let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                // Generate warning for NaN comparison
                if left.is::<NumberConstant>() && left.number_value().is_nan() {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::NanComparison, diagarg![]);
                } else if right.is::<NumberConstant>() && right.number_value().is_nan() {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::NanComparison, diagarg![]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() == right.number_value(), &boolean_type)));
                }
                if left.is::<StringConstant>() && right.is::<StringConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.string_value() == right.string_value(), &boolean_type)));
                }
                if left.is::<BooleanConstant>() && right.is::<BooleanConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.boolean_value() == right.boolean_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::NotEquals |
            Operator::StrictNotEquals => {
                let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                // Generate warning for NaN comparison
                if left.is::<NumberConstant>() && left.number_value().is_nan() {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::NanComparison, diagarg![]);
                } else if right.is::<NumberConstant>() && right.number_value().is_nan() {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::NanComparison, diagarg![]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() != right.number_value(), &boolean_type)));
                }
                if left.is::<StringConstant>() && right.is::<StringConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.string_value() != right.string_value(), &boolean_type)));
                }
                if left.is::<BooleanConstant>() && right.is::<BooleanConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.boolean_value() != right.boolean_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::Lt => {
                let Some(right) = verifier.verify_expression(&exp.right, &VerifierExpressionContext {
                    context_type: Some(left_st.clone()),
                    ..default()
                })? else {
                    return Ok(None);
                };

                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() < right.number_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::Gt => {
                let Some(right) = verifier.verify_expression(&exp.right, &VerifierExpressionContext {
                    context_type: Some(left_st.clone()),
                    ..default()
                })? else {
                    return Ok(None);
                };

                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() > right.number_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::Le => {
                let Some(right) = verifier.verify_expression(&exp.right, &VerifierExpressionContext {
                    context_type: Some(left_st.clone()),
                    ..default()
                })? else {
                    return Ok(None);
                };

                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() <= right.number_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::Ge => {
                let Some(right) = verifier.verify_expression(&exp.right, &VerifierExpressionContext {
                    context_type: Some(left_st.clone()),
                    ..default()
                })? else {
                    return Ok(None);
                };

                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                // Generate warning for unrelated types
                if left_st.is_comparison_between_unrelated_types(&right_st, &verifier.host)? {
                    verifier.add_warning(&exp.location, WhackDiagnosticKind::ComparisonBetweenUnrelatedTypes, diagarg![left_st.clone(), right_st.clone()]);
                }

                if left.is::<NumberConstant>() && right.is::<NumberConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.number_value() >= right.number_value(), &boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&boolean_type)))
            },
            Operator::Instanceof |
            Operator::In |
            Operator::NotIn => {
                let Some(_) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                Ok(Some(verifier.host.factory().create_value(&verifier.host.boolean_type().defer()?)))
            },
            Operator::Is | Operator::IsNot => {
                let Some(_) = verifier.imp_coerce_exp(&exp.right, &verifier.host.class_type().defer()?)? else {
                    return Ok(None);
                };
                Ok(Some(verifier.host.factory().create_value(&verifier.host.boolean_type().defer()?)))
            },
            Operator::As => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &verifier.host.class_type().defer()?)? else {
                    return Ok(None);
                };
                if let Some(mut t) = right.as_type() {
                    t = if t.includes_null(&verifier.host)? || t.includes_undefined(&verifier.host)? {
                        t.clone()
                    } else {
                        verifier.host.factory().create_nullable_type(&t)
                    };
                    return Ok(Some(verifier.host.factory().create_value(&verifier.host.factory().create_value(&t))));
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.factory().create_value(&verifier.host.any_type()))))
            },
            Operator::LogicalAnd => {
                let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                if left.is::<BooleanConstant>() && right.is::<BooleanConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.boolean_value() && right.boolean_value(), &boolean_type)));
                }
                if left_st == boolean_type && left_st == right_st {
                    return Ok(Some(verifier.host.factory().create_value(&boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())))
            },
            Operator::LogicalXor => {
                let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                if left_st == boolean_type && left_st == right_st {
                    return Ok(Some(verifier.host.factory().create_value(&boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())))
            },
            Operator::LogicalOr => {
                let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                    return Ok(None);
                };
                let right_st = right.static_type(&verifier.host);
                let boolean_type = verifier.host.boolean_type().defer()?;

                if left.is::<BooleanConstant>() && right.is::<BooleanConstant>() {
                    return Ok(Some(verifier.host.factory().create_boolean_constant(left.boolean_value() || right.boolean_value(), &boolean_type)));
                }
                if left_st == boolean_type && left_st == right_st {
                    return Ok(Some(verifier.host.factory().create_value(&boolean_type)));
                }
                Ok(Some(verifier.host.factory().create_value(&verifier.host.any_type())))
            },
            Operator::NullCoalescing => {
                let Some(right) = verifier.imp_coerce_exp(&exp.right, &left_st)? else {
                    return Ok(None);
                };

                // Auto escape out of nullable form or propagate non-nullable from
                // the right operand.
                let right_st = right.static_type(&verifier.host);
                if right.is::<ConversionValue>() {
                    if right.conversion_kind() == ConversionKind::NonNullableToNullable {
                        return Ok(Some(verifier.host.factory().create_value(&right_st.base().static_type(&verifier.host))));
                    }
                    if right.conversion_kind() == ConversionKind::AsIsToNullable {
                        return Ok(Some(verifier.host.factory().create_value(&right_st.escape_of_nullable())));
                    }
                }

                Ok(Some(verifier.host.factory().create_value(&left_st)))
            },
            _ => panic!(),
        }
    }

    pub fn verify_conditional_exp(verifier: &mut Subverifier, exp: &ConditionalExpression, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        verifier.verify_expression(&exp.test, &default())?;
        let ctx1 = VerifierExpressionContext {
            context_type: context.context_type.clone(),
            ..default()
        };
        let Some(conseq) = verifier.verify_expression(&exp.consequent, &ctx1)? else {
            verifier.verify_expression(&exp.alternative, &ctx1)?;
            return Ok(None);
        };

        let conseq_st = conseq.static_type(&verifier.host);

        let ctx2 = VerifierExpressionContext {
            context_type: ctx1.context_type.or(Some(conseq_st.clone())),
            ..default()
        };
        let Some(alt) = verifier.verify_expression(&exp.alternative, &ctx2)? else {
            return Ok(None);
        };

        let alt_st = alt.static_type(&verifier.host);

        let _coercion1 = ConversionMethods(&verifier.host).implicit(&alt, &conseq_st, false)?;
        if let Some(_coercion1) = _coercion1 {
            return Ok(Some(verifier.host.factory().create_value(&conseq_st)));
        }

        let _coercion2 = ConversionMethods(&verifier.host).implicit(&conseq, &alt_st, false)?;
        if let Some(_coercion2) = _coercion2 {
            return Ok(Some(verifier.host.factory().create_value(&alt_st)));
        }
        
        verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedTernaryOperands, diagarg![conseq_st, alt_st]);

        Ok(None)
    }

    pub fn verify_seq_exp(verifier: &mut Subverifier, exp: &SequenceExpression) -> Result<Option<Entity>, DeferError> {
        verifier.verify_expression(&exp.left, &default())?;
        let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
            return Ok(None);
        };
        Ok(Some(verifier.host.factory().create_value(&right.static_type(&verifier.host))))
    }

    pub fn verify_reserved_ns_exp(verifier: &mut Subverifier, exp: &ReservedNamespaceExpression) -> Result<Option<Entity>, DeferError> {
        let nskind = match exp {
            ReservedNamespaceExpression::Public(_) => SystemNamespaceKind::Public,
            ReservedNamespaceExpression::Private(_) => SystemNamespaceKind::Private,
            ReservedNamespaceExpression::Protected(_) => SystemNamespaceKind::Protected,
            ReservedNamespaceExpression::Internal(_) => SystemNamespaceKind::Internal,
        };
        let Some(ns) = verifier.scope().search_system_ns_in_scope_chain(nskind) else {
            verifier.add_verify_error(&exp.location(), WhackDiagnosticKind::SystemNamespaceNotFound, diagarg![]);
            return Ok(None);
        };
        Ok(Some(ns.wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_nullable_type_exp(verifier: &mut Subverifier, exp: &NullableTypeExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_type_expression(&exp.base)? else {
            return Ok(None);
        };
        Ok(Some(verifier.host.factory().create_nullable_type(&base).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_non_nullable_type_exp(verifier: &mut Subverifier, exp: &NonNullableTypeExpression) -> Result<Option<Entity>, DeferError> {
        let Some(base) = verifier.verify_type_expression(&exp.base)? else {
            return Ok(None);
        };
        // Marking non nullable on a non-null type as-is results into the same type as-is
        // without the non nullable modifier.
        if !(base.includes_null(&verifier.host)? || base.includes_undefined(&verifier.host)?) {
            return Ok(Some(base.wrap_property_reference(&verifier.host)?));
        }
        Ok(Some(verifier.host.factory().create_non_nullable_type(&base).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_array_type_exp(verifier: &mut Subverifier, exp: &ArrayTypeExpression) -> Result<Option<Entity>, DeferError> {
        let Some(elem_type) = verifier.verify_type_expression(&exp.expression)? else {
            return Ok(None);
        };
        Ok(Some(verifier.host.factory().create_type_after_substitution(&verifier.host.array_type().defer()?, &shared_array![elem_type]).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_tuple_type_exp(verifier: &mut Subverifier, exp: &TupleTypeExpression) -> Result<Option<Entity>, DeferError> {
        let mut elem_types = Vec::<Entity>::new();
        let mut error = false;
        for elem in &exp.expressions {
            let Some(elem_type) = verifier.verify_type_expression(&elem)? else {
                error = true;
                continue;
            };
            elem_types.push(elem_type.clone());
        }
        if error {
            return Ok(None);
        }
        Ok(Some(verifier.host.factory().create_tuple_type(elem_types).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_function_type_exp(verifier: &mut Subverifier, exp: &FunctionTypeExpression) -> Result<Option<Entity>, DeferError> {
        let mut params = Vec::<Rc<SemanticFunctionTypeParameter>>::new();
        let mut error = false;
        let mut last_param_kind = ParameterKind::Required;
        for param_node in &exp.parameters {
            if !last_param_kind.may_be_followed_by(param_node.kind) {
                error = true;
            }
            let param_st: Entity;
            if let Some(param_ty_node) = &param_node.type_expression {
                let Some(param_st_1) = verifier.verify_type_expression(param_ty_node)? else {
                    error = true;
                    last_param_kind = param_node.kind;
                    continue;
                };
                param_st = param_st_1;
            } else {
                // Rest parameter is [*] by default
                param_st = verifier.host.array_type_of_any()?;
            }
            if param_node.kind == ParameterKind::Rest && param_st.array_element_type(&verifier.host)?.is_none() {
                verifier.add_verify_error(&param_node.type_expression.as_ref().unwrap().location(), WhackDiagnosticKind::RestParameterMustBeArray, diagarg![]);
            }
            params.push(Rc::new(SemanticFunctionTypeParameter {
                kind: param_node.kind,
                static_type: param_st,
            }));
            last_param_kind = param_node.kind;
        }

        let result_type: Entity;
        if let Some(result_type_node) = &exp.result_type {
            let Some(result_type_1) = verifier.verify_type_expression(result_type_node)? else {
                return Ok(None);
            };
            result_type = result_type_1;
        } else {
            result_type = verifier.host.any_type();
        }

        if error {
            return Ok(None);
        }

        Ok(Some(verifier.host.factory().create_function_type(params, result_type).wrap_property_reference(&verifier.host)?))
    }

    pub fn verify_assignment_exp(verifier: &mut Subverifier, exp: &AssignmentExpression) -> Result<Option<Entity>, DeferError> {
        if Self::is_destructuring_left_hand_side(&exp.left) && exp.compound.is_none() {
            let Some(right) = verifier.verify_expression(&exp.right, &default())? else {
                AssignmentDestructuringSubverifier::verify_pattern(verifier, &exp.left, &verifier.host.invalidation_entity())?;
                return Ok(None);
            };

            AssignmentDestructuringSubverifier::verify_pattern(verifier, &exp.left, &right)?;

            Ok(Some(verifier.host.factory().create_value(&right.static_type(&verifier.host))))
        } else {
            let ctx1 = VerifierExpressionContext {
                mode: VerifyMode::Write,
                ..default()
            };
            let Some(left) = verifier.verify_expression(&exp.left, &ctx1)? else {
                verifier.verify_expression(&exp.right, &default())?;
                return Ok(None);
            };
            let left_st = left.static_type(&verifier.host);
            let left_st_esc = left_st.escape_of_non_nullable();
            let right = verifier.imp_coerce_exp(&exp.right, &left_st)?;

            if let Some(compound) = exp.compound {
                match compound {
                    Operator::Add |
                    Operator::LogicalAnd |
                    Operator::LogicalXor |
                    Operator::LogicalOr |
                    Operator::NullCoalescing => {},

                    Operator::Subtract |
                    Operator::Multiply |
                    Operator::Divide |
                    Operator::Remainder |
                    Operator::Power |
                    Operator::BitwiseAnd |
                    Operator::BitwiseXor |
                    Operator::BitwiseOr |
                    Operator::ShiftLeft |
                    Operator::ShiftRight |
                    Operator::ShiftRightUnsigned => {
                        if ![verifier.host.any_type(), verifier.host.object_type().defer()?].contains(&left_st_esc)
                        && !verifier.host.numeric_types()?.contains(&left_st_esc)
                        {
                            verifier.add_verify_error(&exp.location, WhackDiagnosticKind::UnrelatedMathOperation, diagarg![left_st.clone()]);
                        }
                    },

                    _ => panic!(),
                }
            }

            if right.is_none() {
                Ok(None)
            } else {
                Ok(Some(verifier.host.factory().create_value(&right.unwrap().static_type(&verifier.host))))
            }
        }
    }

    fn is_destructuring_left_hand_side(exp: &Rc<Expression>) -> bool {
        match exp.as_ref() {
            Expression::Unary(e) => e.operator == Operator::NonNull && Self::is_destructuring_left_hand_side(&e.expression),
            Expression::ArrayLiteral(_) | Expression::ObjectInitializer(_) => true,
            _ => false,
        }
    }

    pub fn verify_function_exp(verifier: &mut Subverifier, exp: &FunctionExpression) -> Result<Option<Entity>, DeferError> {
        let host = verifier.host.clone();
        let kscope = verifier.scope();

        let common = exp.common.clone();
        
        let internal_ns = kscope.search_system_ns_in_scope_chain(SystemNamespaceKind::Internal).unwrap();

        let name = if let Some(name1) = &exp.name {
            verifier.host.factory().create_qname(&internal_ns, name1.0.clone())
        } else {
            verifier.host.empty_empty_qname()
        };

        let name_span = exp.name.as_ref().map(|name| &name.1).unwrap_or(&exp.location);

        let mut partials = verifier.deferred_function_exp.get(&NodeAsKey(common.clone()));
        if partials.is_none() {
            let method = host.factory().create_method_slot(&name, &host.unresolved_entity());
            method.set_is_async(common.contains_await);
            method.set_is_generator(common.contains_yield);

            let act = host.factory().create_activation(&method);
            method.set_activation(Some(act.clone()));

            // The "this" receiver
            if let Some(this_param) = common.signature.this_parameter.clone() {
                let t = verifier.verify_type_expression(&this_param.type_annotation)?.unwrap_or(host.any_type());
                act.set_this(Some(host.factory().create_this_object(&t)));
            } else {
                // Inherit "this" type
                let super_act = kscope.search_activation();
                let super_this_type = super_act.and_then(|a| a.this().map(|this| this.static_type(&verifier.host)));
                act.set_this(Some(host.factory().create_this_object(&super_this_type.unwrap_or(host.any_type()))));
            }

            let partials1 = VerifierFunctionPartials::new(&act, name_span);
            verifier.deferred_function_exp.set(NodeAsKey(common.clone()), partials1.clone());
            partials = Some(partials1);
        }

        let partials = partials.unwrap();
        let activation = partials.activation();

        verifier.inherit_and_enter_scope(&activation);

        if exp.name.is_some() && !activation.properties(&host).borrow().contains_key(&name) {
            let this_func_var = host.factory().create_variable_slot(&name, false, &host.function_type().defer()?);
            this_func_var.set_parent(Some(activation.clone()));
            activation.properties(&host).set(name.clone(), this_func_var);
        }
        
        let mut params: Vec<Rc<SemanticFunctionTypeParameter>> = vec![];
        let mut last_param_kind = ParameterKind::Required;

        if partials.params().is_none() {
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
                                match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false, false) {
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
                                match DestructuringDeclarationSubverifier::verify_pattern(verifier, &param_node.destructuring.destructuring, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false, false) {
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
                                match DestructuringDeclarationSubverifier::verify_pattern(verifier, pattern, &init, false, &mut activation.properties(&host), &internal_ns, &activation, false, false) {
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

        if let Some(result_annot) = common.signature.result_type.as_ref() {
            if partials.result_type().is_none() {
                let result_type = verifier.verify_type_expression(result_annot)?.unwrap_or(host.invalidation_entity());
                partials.set_result_type(Some(result_type));
            }
        }
        /*
        else if !compiler_options.infer_types && partials.result_type().is_none() {
            verifier.add_warning(name_span, WhackDiagnosticKind::ReturnValueHasNoTypeDeclaration, diagarg![]);
            partials.set_result_type(Some(if common.contains_await { host.promise_type_of_any()? } else { host.any_type() }));
        }
        */

        let _ = FunctionCommonSubverifier::verify_function_exp_common(verifier, &common, &partials);

        verifier.set_scope(&kscope);

        Ok(Some(host.factory().create_lambda_object(&activation)?))
    }
}