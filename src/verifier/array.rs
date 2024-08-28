use crate::ns::*;

pub(crate) struct ArraySubverifier;

impl ArraySubverifier {
    pub fn verify_array_literal(verifier: &mut Subverifier, literal: &ArrayLiteral, context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        let context_type = context.context_type.clone().unwrap_or(verifier.host.array_type_of_any()?);
        context_type.defer()?;
        let context_type_esc = context_type.escape_of_nullable_or_non_nullable();

        let object_type = verifier.host.object_type().defer()?;
        let array_type_of_any = verifier.host.array_type_of_any()?;

        if [verifier.host.any_type(), object_type, array_type_of_any].contains(&context_type_esc) {
            for elem in &literal.elements {
                match elem {
                    Element::Rest((exp, _)) => {
                        verifier.imp_coerce_exp(exp, &context_type_esc)?;
                    },
                    Element::Expression(exp) => {
                        verifier.verify_expression(exp, &default())?;
                    },
                    _ => {},
                }
            }
        } else if context_type_esc.is::<TupleType>() {
            let mut elision_found = false;
            let mut i: usize = 0;
            let tuple_type = context_type_esc.clone();
            if literal.elements.len() != tuple_type.element_types().length() {
                verifier.add_verify_error(&literal.location, FlexDiagnosticKind::ArrayLengthNotEqualsTupleLength, diagarg![tuple_type.clone()]);
            }
            for elem in &literal.elements {
                match elem {
                    Element::Elision => {
                        elision_found = true;
                    },
                    Element::Rest((exp, loc)) => {
                        verifier.verify_expression(exp, &default())?;
                        verifier.add_verify_error(loc, FlexDiagnosticKind::UnexpectedRest, diagarg![]);
                    },
                    Element::Expression(exp) => {
                        let element_type = tuple_type.element_types().get(i);
                        if let Some(element_type) = element_type {
                            verifier.imp_coerce_exp(exp, &element_type)?;
                        } else {
                            verifier.verify_expression(exp, &default())?;
                        }
                    },
                }
                i += 1;
            }
            if elision_found {
                verifier.add_verify_error(&literal.location, FlexDiagnosticKind::UnexpectedElision, diagarg![]);
            }
        } else {
            let element_type = context_type_esc.array_element_type(&verifier.host)?;
            if let Some(element_type) = element_type {
                for elem in &literal.elements {
                    match elem {
                        Element::Elision => {},
                        Element::Rest((exp, _)) => {
                            verifier.imp_coerce_exp(exp, &context_type_esc)?;
                        },
                        Element::Expression(exp) => {
                            verifier.imp_coerce_exp(exp, &element_type)?;
                        },
                    }
                }
            } else {
                if !context_type_esc.is::<InvalidationEntity>() {
                    verifier.add_verify_error(&literal.location, FlexDiagnosticKind::UnexpectedArray, diagarg![]);
                }
                for elem in &literal.elements {
                    match elem {
                        Element::Rest((exp, _)) => {
                            verifier.verify_expression(exp, &default())?;
                        },
                        Element::Expression(exp) => {
                            verifier.verify_expression(exp, &default())?;
                        },
                        _ => {},
                    }
                }
            }
        }

        if context_type_esc.is::<InvalidationEntity>() {
            return Ok(Some(verifier.host.invalidation_entity()));
        }

        Ok(Some(verifier.host.factory().create_value(&context_type)))
    }

    pub fn verify_vector_literal(verifier: &mut Subverifier, literal: &VectorLiteral, _context: &VerifierExpressionContext) -> Result<Option<Entity>, DeferError> {
        let element_type = verifier.verify_type_expression(&literal.element_type)?;
        let vector_type = if element_type.is_none() {
            verifier.host.invalidation_entity()
        } else {
            verifier.host.vector_type().defer()?.apply_type(&verifier.host, &verifier.host.vector_type().defer()?.type_params().unwrap(), &shared_array![element_type.clone().unwrap()])
        };

        if !vector_type.is::<InvalidationEntity>() {
            let element_type = element_type.unwrap();
            for elem in &literal.elements {
                match elem {
                    Element::Elision => {
                        panic!();
                    },
                    Element::Rest((exp, _)) => {
                        verifier.imp_coerce_exp(exp, &vector_type)?;
                    },
                    Element::Expression(exp) => {
                        verifier.imp_coerce_exp(exp, &element_type)?;
                    },
                }
            }
        } else {
            for elem in &literal.elements {
                match elem {
                    Element::Elision => {
                        panic!();
                    },
                    Element::Rest((exp, _)) => {
                        verifier.verify_expression(exp, &default())?;
                    },
                    Element::Expression(exp) => {
                        verifier.verify_expression(exp, &default())?;
                    },
                }
            }
        }

        if vector_type.is::<InvalidationEntity>() {
            return Ok(Some(verifier.host.invalidation_entity()));
        }

        Ok(Some(verifier.host.factory().create_value(&vector_type)))
    }
}