use crate::ns::*;

#[path = "flex_diagnostics_texts.rs"]
mod data;

pub struct FlexDiagnostic<'a>(pub &'a Diagnostic);

impl<'a> FlexDiagnostic<'a> {
    pub fn new_syntax_error(location: &Location, kind: FlexDiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) -> Diagnostic {
        let d = Diagnostic::new_syntax_error(location, DiagnosticKind::Expecting, arguments);
        d.set_custom_kind(Some(Rc::new(kind)));
        d
    }

    pub fn new_verify_error(location: &Location, kind: FlexDiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) -> Diagnostic {
        let d = Diagnostic::new_verify_error(location, DiagnosticKind::Expecting, arguments);
        d.set_custom_kind(Some(Rc::new(kind)));
        d
    }

    pub fn new_warning(location: &Location, kind: FlexDiagnosticKind, arguments: Vec<Rc<dyn DiagnosticArgument>>) -> Diagnostic {
        let d = Diagnostic::new_warning(location, DiagnosticKind::Expecting, arguments);
        d.set_custom_kind(Some(Rc::new(kind)));
        d
    }

    pub fn fx_kind(&self) -> Option<FlexDiagnosticKind> {
        if let Some(k) = self.custom_kind() {
            if let Ok(k) = Rc::downcast(k) {
                Some(*k)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn fx_kind_eq(&self, kind: FlexDiagnosticKind) -> bool {
        self.fx_kind().map(|k1| kind == k1).unwrap_or(false)
    }

    pub fn id(&self) -> i32 {
        self.fx_kind().map(|k| k.id()).unwrap_or(self.0.id())
    }

    /// Formats the diagnostic in English.
    pub fn format_english(&self) -> String {
        if self.fx_kind().is_none() {
            return self.0.format_english();
        }
        self.format_with_message(&self.format_message_english(), Some(self.id()))
    }

    pub fn format_message_english(&self) -> String {
        if self.fx_kind().is_none() {
            return self.0.format_message_english();
        }
        self.format_message(&data::DATA)
    }
    
    pub fn format_message(&self, messages: &HashMap<i32, String>) -> String {
        let mut string_arguments: HashMap<String, String> = hashmap!{};
        let mut i = 1;
        for argument in &self.arguments() {
            string_arguments.insert(i.to_string(), argument.to_string());
            i += 1;
        }
        use late_format::LateFormat;
        let Some(msg) = messages.get(&self.id()) else {
            let id = self.id();
            panic!("Message resource is missing for ID {id}");
        };
        msg.late_format(string_arguments)
    }
}

impl<'a> std::ops::Deref for FlexDiagnostic<'a> {
    type Target = &'a Diagnostic;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}