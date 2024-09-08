use crate::ns::*;

pub(crate) struct ArgumentsSubverifier;

impl ArgumentsSubverifier {
    pub fn verify(verifier: &mut Subverifier, arguments: &Vec<Rc<Expression>>, signature: &Entity) -> Result<(), VerifierArgumentsError> {
        let host = verifier.host.clone();
        let sig_params = signature.params();
        let mut sig_params = sig_params.iter();
        let mut rest_elem_type: Option<Entity> = None;
        let mut least_expect_num: usize = 0;
        let mut expect_num: usize = 0;
        let mut exceeds = false;

        for arg in arguments {
            let sig_param = sig_params.next();
            if let Some(sig_param) = sig_param {
                match sig_param.kind {
                    ParameterKind::Rest => {
                        rest_elem_type = Some(map_defer_error(sig_param.static_type.array_element_type(&host))?.unwrap());
                        map_defer_error(verifier.imp_coerce_exp(arg, rest_elem_type.as_ref().unwrap()))?;
                    },
                    _ => {
                        if sig_param.kind == ParameterKind::Required {
                            least_expect_num += 1;
                        }
                        expect_num += 1;
                        map_defer_error(verifier.imp_coerce_exp(arg, &sig_param.static_type))?;
                    },
                }
            } else if let Some(rest_elem_type) = rest_elem_type.as_ref() {
                map_defer_error(verifier.imp_coerce_exp(arg, rest_elem_type))?;
            } else {
                exceeds = true;
                map_defer_error(verifier.verify_expression(arg, &default()))?;
            }
        }

        for sig_param in sig_params {
            if sig_param.kind == ParameterKind::Required {
                least_expect_num += 1;
                expect_num += 1;
            } else if sig_param.kind == ParameterKind::Optional {
                expect_num += 1;
            }
        }

        if exceeds {
            Err(VerifierArgumentsError::ExpectedNoMoreThan(expect_num))
        } else if arguments.len() < least_expect_num {
            Err(VerifierArgumentsError::Expected(least_expect_num))
        } else {
            Ok(())
        }
    }
}

fn map_defer_error<T>(result: Result<T, DeferError>) -> Result<T, VerifierArgumentsError> {
    result.map_err(|_| VerifierArgumentsError::Defer)
}

pub(crate) enum VerifierArgumentsError {
    Defer,
    Expected(usize),
    ExpectedNoMoreThan(usize),
}