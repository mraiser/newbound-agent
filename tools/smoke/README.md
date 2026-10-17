# Browser smokes

The verification that needs a browser and a real instance.

```bash
node tools/smoke/run.mjs --base http://localhost:33199 \
                         --dir /path/to/instance           # every smoke
node tools/smoke/run.mjs scratchpad --base ... --dir ...   # just one
```

Point `--base`/`--dir` at a **disposable** instance (`tools/scratch-instance.md`):
the smokes write fixtures through `dev.code`. Exit code is non-zero if
anything fails, so this is CI-shaped.

| smoke | needs | covers |
|---|---|---|
| `scratchpad` | a disposable instance | the agent chat's scratchpad (agent.stage) with a scripted model: the "pad" grant, stage → eval → snapshot, the snapshot PNG reaching the next model request, mounts persisting across turns, restaging picking up a facet edit, page errors surfacing, and re-mount after a reload |

## What the harness takes care of

These smokes once lived in session scratchpads and died with them. The
harness (`harness.mjs`) fixes what made them un-portable:

- **Record ids are per-instance.** Everything resolves by NAME:
  `harness.agentApi` looks up dev.code's command ids from the store (the
  name predates the editing surface's move from the agent library to dev).
- **Sessions expire** (15 minutes) and a disposable instance restarts often,
  so each run logs in for itself from `<dir>/users/admin.properties`.
- **Paths vary.** Playwright and chromium are resolved from flags, the
  environment, or common locations, with a readable error when missing.

## Notes for writing more

- `t.page(...)` collects console and page errors as failures. A smoke that
  triggers an error on purpose removes that one entry from `t.errors`.
- **Pass every declared param.** Calling a dev.code command without one
  (`author` is the usual miss) panics through the FFI boundary and can take
  the instance down, not just the request.
- Mutating smokes target a `smoketest` library and rewrite their fixtures
  unguarded each run, so they are safe to re-run.
- To exercise the agent chat without an LLM, answer `chat_llm` in-page from
  a queue of canned replies, the way `scratchpad.mjs` does; everything else
  then runs on the production path.

## Retired: the bench smokes

`stage-pools`, `scene-mock`, `flow-editor`, `boot-player`, `wiring` and
`deletion` drove the bench app (its `/bench/` routes and its repo-root
`index.html` + `assets/`), which no longer ships with this repo or the
platform. They were retired on 2026-10-17; git history keeps them.
