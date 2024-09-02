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