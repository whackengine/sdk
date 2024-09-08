#![allow(unused)]

use crate::ns::*;

thread_local! {
    static REPORTED_UNIMPLEMENTED: Cell<bool> = Cell::new(false);
}

pub(crate) struct ControlFlowAnalysisIsUnimplemented;

impl ControlFlowAnalysisIsUnimplemented {
    pub fn unimplemented() {
        if !REPORTED_UNIMPLEMENTED.get() {
            println!("Warning: Control flow analysis does not exist in MXMLC currently.");
            REPORTED_UNIMPLEMENTED.set(true);
        }
    }
}

pub(crate) struct ControlFlowParent<'a> {
    pub parent: ControlFlowBlock,
    pub next_siblings: &'a [Rc<Directive>],
}

pub(crate) struct ControlFlowAnalyser;

impl ControlFlowAnalyser {
    pub fn analyse_directives<'a>(
        _list: &[Rc<Directive>],
        _cfg: &ControlFlowGraph,
        _building_block: &mut Vec<Rc<Directive>>,
        _ascending_parents: &[ControlFlowParent<'a>]
    ) {
        ControlFlowAnalysisIsUnimplemented::unimplemented();
    }
}