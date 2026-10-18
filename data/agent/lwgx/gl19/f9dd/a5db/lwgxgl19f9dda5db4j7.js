// RETIRED 2026-08: the OWNER ADDENDUM moved out of library code into instance
// state. It now lives at runtime:agentchataddendum, `addendum` facet — plain
// text, edited in the workbench or the agent app's prompts tab (no more regex
// surgery on JS source). agent:chat fetches it lazily (ownerAddendum()); this
// control is kept only as a pointer and may be deleted once nothing mounts it.
