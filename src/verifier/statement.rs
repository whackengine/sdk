use crate::ns::*;

pub(crate) struct StatementSubverifier;

impl StatementSubverifier {
    pub fn verify_statements(verifier: &mut Subverifier, list: &[Rc<Directive>]) {
        todo_here();
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