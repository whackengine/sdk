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
            _ => {},
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