use crate::ns::*;

pub(crate) struct StatementSubverifier;

impl StatementSubverifier {
    pub fn verify_statements(verifier: &mut Subverifier, list: &[Rc<Directive>]) {
        for stmt in list.iter() {
            Self::verify_statement(verifier, stmt);
        }
    }

    pub fn verify_statement(verifier: &mut Subverifier, stmt: &Rc<Directive>) {
        match stmt.as_ref() {
            Directive::ExpressionStatement(estmt) => {
                verifier.verify_expression_or_max_cycles_error(&estmt.expression, &Default::default());
            },
            Directive::SuperStatement(supstmt) => {
                Self::verify_super_stmt(verifier, stmt, supstmt)
            },
            Directive::Block(block) => {
                let scope = verifier.host.node_mapping().get(stmt).unwrap();
                verifier.inherit_and_enter_scope(&scope);
                Self::verify_statements(verifier, &block.directives);
                verifier.exit_scope();
            },
            Directive::LabeledStatement(labstmt) => {
                Self::verify_statement(verifier, &labstmt.substatement);
            },
            Directive::IfStatement(ifstmt) => {
                verifier.verify_expression_or_max_cycles_error(&ifstmt.test, &Default::default());
                Self::verify_statement(verifier, &ifstmt.consequent);
                if let Some(alt) = ifstmt.alternative.as_ref() {
                    Self::verify_statement(verifier, alt);
                }
            },
            Directive::SwitchStatement(swstmt) => {
                let host = verifier.host.clone();
                let discriminant = verifier.verify_expression_or_max_cycles_error(&swstmt.discriminant, &Default::default());
                for case in swstmt.cases.iter() {
                    for label in case.labels.iter() {
                        match label {
                            CaseLabel::Case((exp, _)) => {
                                if let Some(discriminant) = discriminant.as_ref() {
                                    verifier.imp_coerce_exp_or_max_cycles_error(exp, &discriminant.static_type(&host));
                                } else {
                                    verifier.verify_expression_or_max_cycles_error(exp, &Default::default());
                                }
                            },
                            CaseLabel::Default(_) => {},
                        }
                    }
                    Self::verify_statements(verifier, &case.directives);
                }
            },
            Directive::SwitchTypeStatement(swstmt) => {
                verifier.verify_expression_or_max_cycles_error(&swstmt.discriminant, &Default::default());
                for case in swstmt.cases.iter() {
                    Self::verify_block(verifier, &case.block);
                }
            },
            Directive::DoStatement(dostmt) => {
                Self::verify_statement(verifier, &dostmt.body);
                verifier.verify_expression_or_max_cycles_error(&dostmt.test, &Default::default());
            },
            Directive::WhileStatement(wstmt) => {
                verifier.verify_expression_or_max_cycles_error(&wstmt.test, &Default::default());
                Self::verify_statement(verifier, &wstmt.body);
            },
            Directive::ForStatement(forstmt) => {
                let host = verifier.host.clone();
                let scope = host.node_mapping().get(&stmt).unwrap();
                verifier.inherit_and_enter_scope(&scope);
                if let Some(ForInitializer::Expression(init)) = forstmt.init.as_ref() {
                    verifier.verify_expression_or_max_cycles_error(&init, &Default::default());
                }
                if let Some(test) = forstmt.test.as_ref() {
                    verifier.verify_expression_or_max_cycles_error(&test, &Default::default());
                }
                if let Some(update) = forstmt.update.as_ref() {
                    verifier.verify_expression_or_max_cycles_error(&update, &Default::default());
                }
                Self::verify_statement(verifier, &forstmt.body);
                verifier.exit_scope();
            },
            Directive::ForInStatement(forstmt) => {
                Self::verify_for_in_stmt(verifier, stmt, forstmt)
            },
            Directive::WithStatement(wstmt) => {
                let host = verifier.host.clone();
                let obj = verifier.verify_expression_or_max_cycles_error(&wstmt.object, &Default::default());
                let scope = host.lazy_node_mapping(stmt, || {
                    if let Some(obj) = obj {
                        host.factory().create_with_scope(&obj)
                    } else {
                        host.factory().create_scope()
                    }
                });
                verifier.inherit_and_enter_scope(&scope);
                Self::verify_statement(verifier, &wstmt.body);
                verifier.exit_scope();
            },
            Directive::ReturnStatement(retstmt) => {
                Self::verify_return_stmt(verifier, stmt, retstmt);
            },
            Directive::ThrowStatement(tstmt) => {
                verifier.verify_expression_or_max_cycles_error(&tstmt.expression, &Default::default());
            },
            Directive::DefaultXmlNamespaceStatement(dxns) => {
                verifier.add_verify_error(&dxns.location, WhackDiagnosticKind::DxnsStatementIsNotSupported, diagarg![]);
                verifier.verify_expression_or_max_cycles_error(&dxns.right, &Default::default());
            },
            Directive::TryStatement(trystmt) => {
                Self::verify_block(verifier, &trystmt.block);
                for catch_clause in trystmt.catch_clauses.iter() {
                    Self::verify_block(verifier, &catch_clause.block);
                }
                if let Some(finally_clause) = trystmt.finally_clause.as_ref() {
                    Self::verify_block(verifier, &finally_clause.block);
                }
            },
            Directive::ConfigurationDirective(cfgdrtv) => {
                let cval = verifier.host.node_mapping().get(stmt).unwrap();
                if cval.is::<BooleanConstant>() && cval.boolean_value() {
                    // Do not just resolve the directive; if it is a block,
                    // resolve it without creating a block scope for it.
                    if let Directive::Block(block) = cfgdrtv.directive.as_ref() {
                        Self::verify_statements(verifier, &block.directives)
                    } else {
                        Self::verify_statement(verifier, &cfgdrtv.directive)
                    }
                }
            },
            Directive::IncludeDirective(incdrtv) => {
                Self::verify_statements(verifier, &incdrtv.nested_directives);
            },
            Directive::DirectiveInjection(inj) => {
                Self::verify_statements(verifier, inj.directives.borrow().as_ref());
            },
            Directive::ClassDefinition(defn) => {
                Self::verify_block(verifier, &defn.block);
            },
            Directive::EnumDefinition(defn) => {
                Self::verify_block(verifier, &defn.block);
            },
            _ => {},
        }
    }

    fn verify_block(verifier: &mut Subverifier, block: &Rc<Block>) {
        let scope = verifier.host.lazy_node_mapping(block, || {
            verifier.host.factory().create_scope()
        });
        verifier.inherit_and_enter_scope(&scope);
        Self::verify_statements(verifier, &block.directives);
        verifier.exit_scope();
    }

    fn verify_return_stmt(verifier: &mut Subverifier, _stmt: &Rc<Directive>, retstmt: &ReturnStatement) {
        let host = verifier.host.clone();
        let act = verifier.scope().search_activation();
        if act.is_none() {
            verifier.add_verify_error(&retstmt.location, WhackDiagnosticKind::IllegalReturnStatement, diagarg![]);
            return;
        }
        let act = act.unwrap();
        let sig = act.of_method().signature(&host);

        if sig.is::<UnresolvedEntity>() {
            if let Some(exp) = retstmt.expression.as_ref() {
                verifier.verify_expression_or_max_cycles_error(&exp, &Default::default());
            }
            return;
        }

        let mut r_t = sig.result_type();

        match r_t.promise_result_type(&host) {
            Ok(Some(prom_r_t)) => {
                r_t = prom_r_t;
            },
            Ok(None) => {},
            Err(_) => {
                verifier.add_verify_error(&retstmt.location, WhackDiagnosticKind::ReachedMaximumCycles, diagarg![]);
                return;
            },
        }

        if let Some(exp) = retstmt.expression.as_ref() {
            verifier.imp_coerce_exp_or_max_cycles_error(exp, &r_t);
        } else if ![host.any_type(), host.void_type()].contains(&r_t) {
            verifier.add_verify_error(&retstmt.location, WhackDiagnosticKind::ReturnValueMustBeSpecified, diagarg![]);
        }
    }

    fn verify_super_stmt(verifier: &mut Subverifier, _stmt: &Rc<Directive>, supstmt: &SuperStatement) {
        let host = verifier.host.clone();
        let mut scope = Some(verifier.scope());
        while let Some(scope1) = scope.as_ref() {
            if scope1.is::<ClassScope>() {
                break;
            }
            scope = scope1.parent();
        }
        if scope.is_none() {
            return;
        }
        let scope = scope.unwrap();
        let class_t = scope.class().extends_class(&host);
        if class_t.is_none() {
            return;
        }
        let class_t = class_t.unwrap();
        let signature;
        if let Some(ctor) = class_t.constructor_method(&host) {
            signature = ctor.signature(&host);
        } else {
            signature = host.factory().create_function_type(vec![], host.void_type());
        }
        match ArgumentsSubverifier::verify(verifier, &supstmt.arguments, &signature) {
            Ok(_) => {},
            Err(VerifierArgumentsError::Expected(n)) => {
                verifier.add_verify_error(&supstmt.location, WhackDiagnosticKind::IncorrectNumArguments, diagarg![n.to_string()]);
            },
            Err(VerifierArgumentsError::ExpectedNoMoreThan(n)) => {
                verifier.add_verify_error(&supstmt.location, WhackDiagnosticKind::IncorrectNumArgumentsNoMoreThan, diagarg![n.to_string()]);
            },
            Err(VerifierArgumentsError::Defer) => {
                verifier.add_verify_error(&supstmt.location, WhackDiagnosticKind::ReachedMaximumCycles, diagarg![]);
            },
        }
    }

    fn verify_for_in_stmt(verifier: &mut Subverifier, stmt: &Rc<Directive>, forstmt: &ForInStatement) {
        let host = verifier.host.clone();
        let scope = host.node_mapping().get(&stmt).unwrap();

        if let ForInBinding::Expression(dest) = &forstmt.left {
            // Resolve object key-values
            let obj = verifier.verify_expression_or_max_cycles_error(&forstmt.right, &Default::default());
            let mut kv_types = (host.any_type(), host.any_type());
            if let Some(obj) = obj.as_ref() {
                let kv_types_1 = StatementSubverifier::for_in_kv_types(&host, obj);
                if kv_types_1.is_err() {
                    verifier.add_verify_error(&forstmt.right.location(), WhackDiagnosticKind::ReachedMaximumCycles, diagarg![]);
                    return;
                }
                let kv_types_1 = kv_types_1.unwrap();
                if let Some(kv_types_1) = kv_types_1 {
                    kv_types = kv_types_1;
                } else {
                    verifier.add_verify_error(&forstmt.right.location(), WhackDiagnosticKind::CannotIterateType, diagarg![obj.unwrap().static_type(&host)]);
                }
            }
            let mut expected_type = if forstmt.each { kv_types.1 } else { kv_types.0 };

            // Resolve destination
            let dest = verifier.verify_expression_or_max_cycles_error(dest, &VerifierExpressionContext {
                mode: VerifyMode::Write,
                ..default()
            });
            if let Some(dest) = dest {
                let dest_t = dest.static_type(&host);

                // If the expected type is not * or Object, then
                // the destination type must be either:
                // - equals to or base type of the expected type (non nullable)
                // - the * type
                // - the Object type (non nullable)
                // - a number type if the expected type is a number type
                let obj_t = host.object_type().defer()?;
                if ![host.any_type(), obj_t.clone()].contains(&expected_type) {
                    let anty = dest_t.escape_of_non_nullable();
                    let exty = expected_type.escape_of_non_nullable();

                    let eq = exty.is_equals_or_subtype_of(&anty, &host)?;
                    let any_or_obj = anty == host.any_type() || anty == obj_t;
                    let num = host.numeric_types()?.contains(&anty) && host.numeric_types()?.contains(&exty);

                    if !(eq || any_or_obj || num) {
                        verifier.add_verify_error(&dest.location(), WhackDiagnosticKind::ExpectedToIterateType, diagarg![exty]);
                    }
                }

                expected_type = dest_t;
            }
        }
        verifier.inherit_and_enter_scope(&scope);
        Self::verify_statement(verifier, &forstmt.body);
        verifier.exit_scope();
    }

    pub fn for_in_kv_types(host: &Database, obj: &Entity) -> Result<Option<(Entity, Entity)>, DeferError> {
        let t = obj.static_type(host).escape_of_non_nullable();
        let obj_t = host.object_type().defer()?;
        // * or Object
        if [host.any_type(), obj_t].contains(&t) {
            return Ok(Some((host.any_type(), host.any_type())));
        }
        // [T]
        if let Some(elem_t) = t.array_element_type(host)? {
            return Ok(Some((host.number_type().defer()?, elem_t)));
        }
        // Vector.<T>
        if let Some(elem_t) = t.vector_element_type(host)? {
            return Ok(Some((host.number_type().defer()?, elem_t)));
        }
        // ByteArray
        if t == host.byte_array_type().defer()? {
            let num_t = host.number_type().defer()?;
            return Ok(Some((num_t.clone(), num_t)));
        }
        // Dictionary
        if t == host.dictionary_type().defer()? {
            return Ok(Some((host.any_type(), host.any_type())));
        }
        let proxy_t = host.proxy_type().defer()?;
        // Proxy
        if t == proxy_t || t.is_subtype_of(&proxy_t, host)? {
            return Ok(Some((host.string_type().defer()?, host.any_type())));
        }
        // XML or XMLList
        if t == host.xml_type().defer()? || t == host.xml_list_type().defer()? {
            return Ok(Some((host.number_type().defer()?, host.xml_type())));
        }

        Ok(None)
    }
}