use crate::ns::*;

#[derive(Clone)]
pub(crate) struct VerifierFunctionPartials(Rc<VerifierFunctionPartials1>);

impl VerifierFunctionPartials {
    pub fn new(activation: &Entity, name_span: &Location) -> Self {
        Self(Rc::new(VerifierFunctionPartials1 {
            name_span: name_span.clone(),
            activation: activation.clone(),
            params: RefCell::new(None),
            result_type: RefCell::new(None),
            signature: RefCell::new(None),
        }))
    }

    pub fn activation(&self) -> Entity {
        self.0.activation.clone()
    }

    pub fn name_span(&self) -> Location {
        self.0.name_span.clone()
    }

    pub fn params(&self) -> std::cell::Ref<Option<Vec<Rc<SemanticFunctionTypeParameter>>>> {
        self.0.params.borrow()
    }

    pub fn set_params(&self, list: Option<Vec<Rc<SemanticFunctionTypeParameter>>>) {
        self.0.params.replace(list);
    }

    pub fn result_type(&self) -> Option<Entity> {
        self.0.result_type.borrow().as_ref().cloned()
    }

    pub fn set_result_type(&self, entity: Option<Entity>) {
        self.0.result_type.replace(entity);
    }

    pub fn signature(&self) -> Option<Entity> {
        self.0.signature.borrow().as_ref().cloned()
    }

    pub fn set_signature(&self, entity: Option<Entity>) {
        self.0.signature.replace(entity);
    }
}

struct VerifierFunctionPartials1 {
    pub activation: Entity,
    pub name_span: Location,
    pub params: RefCell<Option<Vec<Rc<SemanticFunctionTypeParameter>>>>,
    pub result_type: RefCell<Option<Entity>>,
    pub signature: RefCell<Option<Entity>>,
}

pub(crate) struct FunctionCommonSubverifier;

impl FunctionCommonSubverifier {
    pub fn verify_function_exp_common(verifier: &mut Subverifier, common: &Rc<FunctionCommon>, partials: &VerifierFunctionPartials) -> Result<(), DeferError> {
        let host = verifier.host.clone();
        let activation =  partials.activation();
        let method = activation.of_method();
        verifier.set_scope(&activation);

        let name_span = partials.name_span();

        // Attempt to create signature
        let mut signature: Option<Entity> = partials.signature();
        if signature.is_none() && partials.result_type().is_some() {
            let mut result_type = partials.result_type().unwrap(); 

            if common.contains_await && !result_type.promise_result_type(&host)?.is_some() {
                verifier.add_verify_error(&name_span, WhackDiagnosticKind::ReturnTypeDeclarationMustBePromise, diagarg![]);
                result_type = host.promise_type().defer()?.apply_type(&host, &host.promise_type().defer()?.type_params().unwrap(), &shared_array![host.invalidation_entity()])
            }

            let signature1 = host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type);
            partials.set_signature(Some(signature1.clone()));
            signature = Some(signature1);
        }

        // Set the activation method's signature to the last obtained signature if any.
        if let Some(signature) = signature.clone() {
            method.set_signature(&signature);
        }

        if signature.is_none() {
            return Err(DeferError(None));
        }

        // Resolve directives and then statements, or just the expression body.
        match &common.body {
            Some(FunctionBody::Block(block)) => {
                let block_scope = host.factory().create_scope();
                verifier.inherit_and_enter_scope(&block_scope);
                DirectiveSubverifier::verify_directives(verifier, &block.directives)?;
                StatementSubverifier::verify_statements(verifier, &block.directives);
                verifier.exit_scope();
            },
            Some(FunctionBody::Expression(exp)) => {
                if let Some(result_type) = partials.result_type() {
                    verifier.imp_coerce_exp(exp, &result_type)?;
                } else {
                    verifier.verify_expression(exp, &default())?;
                }
            },
            None => {},
        }

        // Analyse the control flow (for block only).
        if let Some(FunctionBody::Block(block)) = &common.body {
            ControlFlowAnalyser::analyse_directives(&block.directives, &activation.control_flow_graph(), &mut vec![], &[]);
        }

        // If the signature is fully resolved, ensure all code paths return a value.
        // Result types that do not require a return value are
        // `*`, `void`, `Promise.<*>`, and `Promise.<void>`.
        if let Some(_signature) = partials.signature() {
            ControlFlowAnalysisIsUnimplemented::unimplemented();
        // If the signature is not fully resolved due to unknown result type,
        // collect the result value types returned from all code paths,
        // ensure the result of all code paths implicitly coerce to the first code path's
        // result's type, and construct the signature into the signature local.
        //
        // If the result type does not match a Promise for an asynchronous method,
        // change it to Promise.<INVALIDATED> and report an error.
        } else {
            let promise_type = host.promise_type().defer()?;

            // let mut result_type = Self::deduce_result_type(verifier, None);
            verifier.add_warning(&name_span, WhackDiagnosticKind::ReturnTypeInferenceIsNotImplemented, diagarg![]);
            let mut result_type = if common.contains_await { host.promise_type_of_any()? } else { host.any_type() };

            if common.contains_await {
                if result_type.is::<InvalidationEntity>() {
                    result_type = promise_type.apply_type(&host, &promise_type.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                } else if result_type.promise_result_type(&host)?.is_none() {
                    verifier.add_verify_error(&name_span, WhackDiagnosticKind::ReturnTypeDeclarationMustBePromise, diagarg![]);
                    result_type = promise_type.apply_type(&host, &promise_type.type_params().unwrap(), &shared_array![host.invalidation_entity()]);
                }
            }

            signature = Some(host.factory().create_function_type(partials.params().as_ref().unwrap().clone(), result_type));
        }

        // Set the activation method's signature to the last obtained signature. 
        method.set_signature(&signature.unwrap());

        Ok(())
    }

    
    pub fn verify_function_definition_common(verifier: &mut Subverifier, common: &Rc<FunctionCommon>, partials: &VerifierFunctionPartials) -> Result<(), DeferError> {
        let host = verifier.host.clone();
        let activation =  partials.activation();
        verifier.set_scope(&activation);

        // Resolve directives and then statements, or just the expression body.
        match &common.body {
            Some(FunctionBody::Block(block)) => {
                let block_scope = host.factory().create_scope();
                verifier.inherit_and_enter_scope(&block_scope);
                DirectiveSubverifier::verify_directives(verifier, &block.directives)?;
                verifier.exit_scope();
            },
            _ => {},
        }

        Ok(())
    }

    
    pub fn verify_function_definition_common_statements_only(verifier: &mut Subverifier, common: &Rc<FunctionCommon>, partials: &VerifierFunctionPartials) {
        let host = verifier.host.clone();
        let activation =  partials.activation();
        verifier.set_scope(&activation);

        // Resolve directives and then statements, or just the expression body.
        match &common.body {
            Some(FunctionBody::Block(block)) => {
                let block_scope = host.factory().create_scope();
                verifier.inherit_and_enter_scope(&block_scope);
                StatementSubverifier::verify_statements(verifier, &block.directives);
                verifier.exit_scope();
            },
            Some(FunctionBody::Expression(exp)) => {
                if let Some(result_type) = partials.result_type() {
                    verifier.imp_coerce_exp_or_max_cycles_error(exp, &result_type);
                } else {
                    verifier.verify_expression_or_max_cycles_error(exp, &default());
                }
            },
            None => {},
        }

        // Analyse the control flow (for block only).
        if let Some(FunctionBody::Block(block)) = &common.body {
            ControlFlowAnalyser::analyse_directives(&block.directives, &activation.control_flow_graph(), &mut vec![], &[]);
        }

        // Ensure all code paths return a value.
        // Result types that do not require a return value are
        // `*`, `void`, `Promise.<*>`, and `Promise.<void>`.
        if let Some(_signature) = partials.signature() {
            ControlFlowAnalysisIsUnimplemented::unimplemented();
        }

        // Cleanup the VerifierFunctionPartials cache from Subverifier.
        verifier.function_definition_partials.remove(&NodeAsKey(common.clone()));
    }
}