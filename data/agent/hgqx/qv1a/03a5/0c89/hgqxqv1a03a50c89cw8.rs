// The board is per-item records (agent.plan.store); the first read on an
// instance imports the legacy kb.plan memory facet.
crate::agent::plan::compat::board()