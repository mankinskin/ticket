pub(crate) const STATE_OPS: &[(&str, &str)] = &[
    ("→ open", "open"),
    ("→ planned", "planned"),
    ("→ in-impl", "in-implementation"),
    ("→ in-review", "in-review"),
];

pub(crate) const PRIORITY_OPS: &[(&str, &str)] = &[
    ("↑ critical", "critical"),
    ("↑ high", "high"),
    ("↓ medium", "medium"),
    ("↓ low", "low"),
];

#[derive(Clone, PartialEq)]
pub(crate) enum BulkOp {
    SetState(String),
    Close,
    Cancel,
    SetPriority(String),
}

#[derive(Clone)]
pub(crate) struct CommandResult {
    pub id: String,
    pub ok: bool,
    pub message: String,
}
