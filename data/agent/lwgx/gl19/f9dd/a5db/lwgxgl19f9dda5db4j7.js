// RETIRED 2026-08: the OWNER ADDENDUM moved out of library code into instance
// state. It is now a DATA RECORD in the instance-local `runtime` library — id
// "agentchataddendum", data.addendum = plain text — NOT a control (the runtime
// library holds instance data/settings, never controls). Read via app.app.read
// (skip if the library/record is absent — the addendum is optional), written
// via app.app.write (created on first save in the agent app's prompts tab).
// agent:chat fetches it lazily (ownerAddendum()); this control is kept only as
// a pointer and may be deleted once nothing mounts it.
