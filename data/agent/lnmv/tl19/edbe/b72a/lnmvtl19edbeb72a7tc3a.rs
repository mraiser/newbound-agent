// INITIALIZE AGENT MEMORY
{
  let mut g = DataStore::globals();
  if !g.has("AGENT_MEMORY") { g.put_object("AGENT_MEMORY", DataObject::new()); }
}

// --- helpers ----------------------------------------------------------------
// Plan activity is derived at real execution boundaries, never model-authored
// status. The prompt-only public command stays source-compatible: step_item
// hands its run metadata through a one-shot dispatch record.
fn activity_emit(run_id: &str, event: DataObject) {
    if run_id.is_empty() { return; }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let cmd = Command::lookup("agent", "plan", "activity_emit");
        let mut a = DataObject::new(); a.put_string("run_id", run_id);
        a.put_object("event", event); let _ = cmd.execute(a);
    }));
}
fn activity_field(run_id: &str, kind: &str, phase: &str, detail: &str, round: i64) {
    let mut e = DataObject::new(); e.put_string("kind", kind);
    e.put_string("phase", phase); e.put_string("state", phase);
    e.put_string("detail", detail); e.put_int("round", round);
    activity_emit(run_id, e);
}
fn safe_target(args: &str) -> String {
    // Targets only: raw arguments may contain secrets, source, or large data.
    match DataObject::try_from_string(&format!("{{\"a\":{}}}", args)) {
        Ok(w) => if let Ok(a) = w.try_get_object("a") {
            let target = if a.has("lib") && a.has("ctl") && a.has("cmd") {
                format!("{}.{}.{}", a.get_string("lib"), a.get_string("ctl"), a.get_string("cmd"))
            } else if a.has("lib") && a.has("ctl") && a.has("facet") {
                format!("{}.{}:{}", a.get_string("lib"), a.get_string("ctl"), a.get_string("facet"))
            } else if a.has("claim") { a.get_string("claim") }
            else if a.has("name") { a.get_string("name") }
            else { "arguments available in the bounded run record".to_string() };
            target.chars().take(300).collect()
        } else { "invalid argument object".to_string() },
        Err(_) => "invalid argument JSON".to_string(),
    }
}
let mut run_id = String::new();
let mut run_turn = 0i64;
let mut dispatch = DataObject::new();
{
    let mut g = DataStore::globals();
    if let Ok(d) = g.try_get_object("AGENT_PLAN_DISPATCH") {
        if d.try_get_string("prompt").unwrap_or_default() == prompt {
            run_id = d.try_get_string("run_id").unwrap_or_default();
            run_turn = d.try_get_int("turn").unwrap_or(0);
            dispatch = d.deep_copy();
            g.remove_property("AGENT_PLAN_DISPATCH");
        }
    }
}
if !run_id.is_empty() {
    let mut e = DataObject::new(); e.put_string("kind", "start");
    e.put_string("claim", &dispatch.try_get_string("claim").unwrap_or_default());
    e.put_int("turn", run_turn);
    if let Ok(c) = dispatch.try_get_object("context") { e.put_object("context", c); }
    activity_emit(&run_id, e);
}
// Covers early returns and unwinding as well as the normal finish path. The
// process may still be alive when the TURN is not; step_item's panic path is a
// second safety net for failures before this guard can be constructed.
struct ActivityGuard { run_id: String, closed: bool }
impl Drop for ActivityGuard {
    fn drop(&mut self) {
        if self.closed || self.run_id.is_empty() { return; }
        let mut e = DataObject::new(); e.put_string("kind", "finish");
        e.put_string("phase", "interrupted"); e.put_string("state", "interrupted");
        e.put_string("outcome", "ERROR");
        e.put_string("detail", "The model/tool loop exited before its normal finish boundary; resume from the last saved observation, not an assumption of completion.");
        activity_emit(&self.run_id, e);
    }
}
let mut activity_guard = ActivityGuard { run_id: run_id.clone(), closed: run_id.is_empty() };
fn extract_compile_errors(raw: &str) -> String {
  // Compile failures arrive as {"status":"err","kind":"compile_error","msg":"error[E0382]: ..."}
  // serialized to JSON, so newlines inside msg are escaped literals. Unescape, then keep
  // everything from the first rustc error marker onward %E2%80%94 dev.dev.compile already
  // pre-filters stderr to error blocks, so nothing after that point is noise.
  let expanded = raw.replace("\\n", "\n").replace("\\\"", "\"");
  match expanded.find("error[").or_else(|| expanded.find("error:")) {
    Some(i) => expanded[i..].to_string(),
    None => expanded
  }
}

fn clamp(s: &str, budget: usize) -> String {
  if s.len() <= budget { return s.to_string(); }
  // Head-biased: rustc's FIRST error is the root cause; later ones usually cascade.
  let head_len = budget * 2 / 3;
  let tail_len = budget - head_len;
  let mut h = head_len; while !s.is_char_boundary(h) { h -= 1; }
  let mut t = s.len() - tail_len; while !s.is_char_boundary(t) { t += 1; }
  format!("{}\n...[{} bytes omitted]...\n{}", &s[..h], t - h, &s[t..])
}
// -----------------------------------------------------------------------------

let sys_prompt = {
  // The PLATFORM-KNOWLEDGE CORE is store-resident (agent.prompts `prompt`
  // facet - docs/prompting.md in the bench repo): ONE journaled source of
  // truth shared with the bench notebook's agentloop. A missing core
  // FAILS LOUD - a silently thin prompt regresses the confabulation
  // lesson - and code + data ride the same repo, so a pull cannot
  // desync them.
  let pstore = DataStore::new();
  // Resolve the control id from the library index directly. An api-struct
  // accessor bakes in whichever library layout api.rs was last generated
  // from (that is the E0609 the dev.code move produced), and a cross-library
  // command call would tie this to dev.editcontrol surviving the cruft
  // audit. The index is the thing that actually holds the answer.
  let pid = {
    let list = pstore.get_data("agent", "controls").get_object("data").get_array("list");
    let mut found = String::new();
    for i in 0..list.len() {
      let it = list.get_object(i);
      if it.get_string("name") == "prompts" { found = it.get_string("id"); break; }
    }
    found
  };
  let missing = !pstore.exists("agent", &pid) || {
    let d = pstore.get_data("agent", &pid).get_object("data");
    !d.has("prompt") || d.get_string("prompt").trim().is_empty()
  };
  if missing {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", "agent.prompts `prompt` facet unavailable - the curriculum is store-resident (docs/prompting.md); pull canon");
    return o;
  }
  let core = pstore.get_data("agent", &pid).get_object("data").get_string("prompt");
  let shell = r#"

The Newbound Commands in the `code` control of the `dev` library are available to you as native tools. Call them through the tool-calling interface - do not describe tool calls in text. Tool names are in the format library-control-command (e.g. dev-code-read_command).

Sub-Agent Recursion
Inside an evaluate_rust script you can call the LLM itself, for map-reduce over data too large for this conversation:
```Rust
use flowlang::datastore::DataStore;
use crate::agent::llm::ask_llm::ask_llm;
use ndata::data::Data;
let store = DataStore::new();
let prompt = format!("Summarize this data: {}", store.get_data("runtime","exported_data").get_object("data").to_string());
let summary = ask_llm(prompt, Data::DString("Answer with the summary only.".to_string()));
let _ = std::fs::remove_file(store.get_data_file("runtime", "exported_data"));
summary
```
Use this to build multi-step pipelines, perform map-reduce tasks, or spin up sub-agents, returning only the final, concise result.
"#;
  core + shell
};
// MEMORY INDEX (docs/memory.md, federated per docs/one-memory-cycle.md A1):
// every library's controls, any control carrying a memory facet is a domain,
// labeled lib.ctl - the control's manual, one read away. kb's controls are
// domains by definition and always listed. One-shot loop, so no staleness
// machinery: the curriculum says to verify source hashes on read.
let mut sys_prompt = sys_prompt;
{
  let mstore = DataStore::new();
  let mut idx = String::from("\n\nMEMORY INDEX (recall with dev-code-read_control_facet lib:<lib> ctl:<ctl> facet:\"memory\"):\n");
  let mut any = false;
  let mut libs: Vec<String> = Vec::new();
  if let Ok(rd) = std::fs::read_dir(&mstore.root) {
    for e in rd.flatten() {
      if e.path().is_dir() {
        if let Ok(n) = e.file_name().into_string() { libs.push(n); }
      }
    }
  }
  libs.sort();
  for lib in libs {
    if !mstore.exists(&lib, "controls") { continue; }
    let list = mstore.get_data(&lib, "controls").get_object("data").get_array("list");
    let is_brain = lib == "kb";
    for i in 0..list.len() {
      let item = list.get_object(i);
      if !item.has("name") || !item.has("id") { continue; }
      let name = item.get_string("name");
      let id = item.get_string("id");
      if !mstore.exists(&lib, &id) { continue; }
      let dd = mstore.get_data(&lib, &id).get_object("data");
      if !dd.has("memory") && !is_brain { continue; }
      let desc = if dd.has("desc") { dd.get_string("desc") } else { String::new() };
      let tags = if dd.has("tags") { dd.get_string("tags") } else { String::new() };
      let n: i64 = if dd.has("memory") {
        match ndata::dataobject::DataObject::try_from_string(&format!("{{\"a\":{}}}", dd.get_string("memory"))) {
          Ok(w) => match w.try_get_array("a") { Ok(a) => a.len() as i64, Err(_) => -1 },
          Err(_) => -1,
        }
      } else { 0 };
      let count = if n >= 0 { format!(" ({})", n) } else { String::new() };
      let tagpart = if tags.is_empty() { String::new() } else { format!(" [{}]", tags) };
      idx.push_str(&format!("- {}.{}{}{} - {}\n", lib, name, count, tagpart, desc));
      any = true;
    }
  }
  if any { sys_prompt.push_str(&idx); }
}

// Build the OpenAI-format tools array from the MCP descriptors
// The write API lives at dev.code (CONTRACT 6.6). Look the command up by
// NAME: an api-struct accessor bakes in whichever library layout api.rs
// was last generated from, so it breaks on regeneration after a move;
// Command::lookup resolves against the store at runtime and compiles
// against any generation of api.rs.
let mcp_tools = {
    let mut d = DataObject::new();
    d.put_string("lib", "dev");
    d.put_string("ctl", "code");
    Command::lookup("dev", "code", "list_commands").execute(d)
        .expect("dev.code.list_commands failed").get_array("a")
};
let mut tools = DataArray::new();
for t in mcp_tools.objects() {
  let t = t.object();
  if !t.has("name") { continue; }
  // Runtime telemetry is emitted by the execution substrate, never chosen by
  // the model. Read-side activity/history remains available for continuity.
  if t.get_string("name") == "agent-plan-activity_emit" { continue; }
  let mut f = DataObject::new();
  f.put_string("name", &t.get_string("name"));
  if t.has("description") { f.put_string("description", &t.get_string("description")); }
  if t.has("inputSchema") { f.put_object("parameters", t.get_object("inputSchema")); }
  else if t.has("input_schema") { f.put_object("parameters", t.get_object("input_schema")); }
  else {
    let mut s = DataObject::new();
    s.put_string("type", "object");
    s.put_object("properties", DataObject::new());
    f.put_object("parameters", s);
  }
  let mut w = DataObject::new();
  w.put_string("type", "function");
  w.put_object("function", f);
  tools.push_object(w);
}

// Seed the conversation
let mut messages = DataArray::new();
let mut m = DataObject::new();
m.put_string("role", "system");
m.put_string("content", &sys_prompt);
messages.push_object(m);
let mut m = DataObject::new();
m.put_string("role", "user");
m.put_string("content", &prompt);
messages.push_object(m);

let mut final_answer = String::new();
let mut iterations = 0;
// The round limit is the agent's TOOL_BUDGET (runtime/agent/botd.properties,
// system.apps.agent.runtime) - the SAME knob the chat agentloop uses, so the
// executive's reasoning and the chat agent run on one setting, not a hardcoded
// 24 (owner, 2026-08: a 16x mismatch let real plan work die at 24 while
// TOOL_BUDGET=400 sat unused). Read LIVE per call so a botd.properties edit
// takes effect without a restart; floor 24 so a tiny/absent value can't make
// the cap trivially trippable; capped 1000 as a guard.
let max_iterations: i64 = {
    let mut b: i64 = 24;
    let system = DataStore::globals().get_object("system");
    if system.has("apps") {
        let apps = system.get_object("apps");
        if apps.has("agent") {
            let agent = apps.get_object("agent");
            if agent.has("runtime") {
                let rt = agent.get_object("runtime");
                if rt.has("TOOL_BUDGET") {
                    if let Ok(n) = rt.get_string("TOOL_BUDGET").trim().parse::<i64>() {
                        if n > 0 { b = n.min(1000); }
                    }
                }
            }
        }
    }
    b.max(24)
};
let mut dirty = String::new();    // "lib-ctl-cmd" modified but not yet executed
let mut nudged = false;           // one verify-nudge per dirty cycle
let mut last_sig = String::new(); // repeated-identical-call detection

let mut consecutive_failures = 0;
let mut repeat_count = 0;
let mut last_err_head = String::new();
let mut executed_calls = 0i64;
let mut error_count = 0i64;
let mut tool_calls_seen = 0i64;
let mut pending_checkpoint = String::new();
let mut checkpoint_reason = String::new();
let mut last_action = String::new();
let mut last_observation = String::new();

loop {
    if iterations >= max_iterations {
        // Budget exhausted: do NOT discard the work. Ask the model to compress
        // its own progress into a resumable checkpoint (one bounded extra
        // turn), so step_item can journal WHERE IT GOT TO and the next tick
        // resumes instead of restarting from scratch (owner, 2026-08: the cap
        // was a 15-minute amnesia loop - same reads, same cap, same error).
        messages.push_object({
            let mut m = DataObject::new();
            m.put_string("role", "user");
            m.put_string("content", "You are OUT of tool budget for this turn. Do NOT call any more tools. Reply with ONLY plain text, no tool calls - a checkpoint another run can resume from, in exactly this shape:\nPROGRESS: what you learned, decided, and changed (be specific: files/commands touched, key findings).\nNEXT: the single next action to take.\nBLOCKERS: anything preventing completion, or 'none'.\nKeep the WHOLE checkpoint under ~1500 characters - this is the only context the next turn gets, so every word must earn its place. Compress; do not truncate mid-thought.");
            m
        });
        let sum = chat_llm(messages.clone(), tools.clone());
        let txt = if sum.get_string("kind") == "text" {
            sum.get_string("content")
        } else {
            String::new()
        };
        checkpoint_reason = format!("Tool-round budget exhausted at {}/{}; a resumable checkpoint was requested.", iterations, max_iterations);
        final_answer = if txt.trim().is_empty() {
            "Error: tool budget exhausted and no progress summary could be produced.".to_string()
        } else {
            pending_checkpoint = clamp(&format!("[budget checkpoint - incomplete, resume next turn]\n{}\nLAST ACTION: {}\nLAST RESULT:\n{}", txt.trim(),
                if last_action.is_empty() { "none".to_string() } else { last_action.clone() },
                if last_observation.is_empty() { "none".to_string() } else { clamp(&last_observation, 3000) }), 8000);
            pending_checkpoint.clone()
        };
        break;
    }
    iterations += 1;
    activity_field(&run_id, "model_wait", "waiting for model response",
        "The model has the in-turn conversation; a response or tool request has not arrived yet.", iterations);

    let resp = chat_llm(messages.clone(), tools.clone());
    if !run_id.is_empty() {
        let mut e = DataObject::new(); e.put_string("kind", "model_received");
        e.put_string("phase", "model response received"); e.put_string("state", "processing model response");
        e.put_int("round", iterations); e.put_int("max_rounds", max_iterations);
        activity_emit(&run_id, e);
    }
    let kind = resp.get_string("kind");
    println!("TL[{}] kind={}", iterations, kind);
  
    if kind == "error" {
        error_count += 1;
        activity_field(&run_id, "model_error", "model backend error", &resp.get_string("content"), iterations);
        final_answer = format!("LLM backend error: {}", resp.get_string("content"));
        checkpoint_reason = "Model backend error ended the run before a model-authored summary.".to_string();
        break;
    }

    if kind == "text" {
        let content = resp.get_string("content");
        activity_field(&run_id, "model_text", "model responded", "Received a text response; evaluating verification and status.", iterations);
        println!("TL[{}] text {} chars; dirty='{}' nudged={}", iterations, content.len(), dirty, nudged);
        // Verify nudge: the workflow's EXECUTE step, enforced once per modification cycle.
        if !dirty.is_empty() && !nudged {
            messages.push_object(resp.get_object("assistant_message"));
            let mut m = DataObject::new();
            m.put_string("role", "user");
            m.put_string("content", &format!("You modified `{}` but never executed it. Follow your workflow: verify with dev-code-invoke_command (or dev-code-evaluate_rust for a test harness) before giving your final answer. If you already verified it another way, state how and answer.", dirty));
            messages.push_object(m);
            nudged = true;
            activity_field(&run_id, "verification_required", "verification pending", &format!("{} was modified but has not been executed yet.", dirty), iterations);
            continue;
        }
        final_answer = content;
        break;
    }

    // kind == "tool_calls"
    if let Ok(am) = resp.try_get_object("assistant_message") {
        if let Ok(content) = am.try_get_string("content") {
            if !content.trim().is_empty() {
                activity_field(&run_id, "assistant_progress", "assistant progress", &clamp(&content, 2000), iterations);
            }
        }
    }
    messages.push_object(resp.get_object("assistant_message"));
    let calls = resp.get_array("tool_calls");
    for c in calls.objects() {
        let c = c.object();
        let name = c.get_string("name");
        let args_str = c.get_string("arguments");
        let call_id = c.get_string("id");
        tool_calls_seen += 1;
        let target = safe_target(&args_str);
        let mut requested = DataObject::new(); requested.put_string("kind", "tool_requested");
        requested.put_string("phase", "tool requested"); requested.put_string("tool", &name);
        requested.put_string("target", &target); requested.put_int("round", iterations);
        requested.put_int("tool_calls", tool_calls_seen); requested.put_string("detail", &call_id);
        let mut pending = DataObject::new(); pending.put_string("tool", &name);
        pending.put_string("target", &target); pending.put_int("round", iterations);
        requested.put_object("pending_tool", pending); activity_emit(&run_id, requested);
        let sig = format!("{}::{}", &name, &args_str);
        let args_head: String = args_str.chars().take(200).collect();
        println!("TL[{}] -> {} args: {}", iterations, name, args_head.replace('\n', " "));

        let duplicate = sig == last_sig;
        let observation = if sig == last_sig {
            repeat_count += 1;
            let mut e = DataObject::new(); e.put_string("kind", "tool_skipped");
            e.put_string("phase", "duplicate call suppressed"); e.put_string("tool", &name);
            e.put_string("target", &target); e.put_int("round", iterations);
            e.put_int("duplicate_streak", repeat_count); e.put_int("streak", repeat_count);
            e.put_boolean("ok", false); e.put_string("detail", "Same tool and arguments as the preceding call; not executed again.");
            activity_emit(&run_id, e);
            if repeat_count >= 3 {
                final_answer = format!("Aborting: the model repeated the identical call to {} {} times despite being told the result will not change. Last real result from this tool is above in the transcript. This usually means the underlying operation is failing and the model has no viable alternative.", name, repeat_count + 1);
                checkpoint_reason = "Repeated-identical-call guard stopped this run.".to_string();
                break;
            }
            "You already called this exact tool with these exact arguments; the result will not change. Adjust the arguments or take a different approach.".to_string()
        } else {
            repeat_count = 0;
            last_sig = sig;
            executed_calls += 1;
            activity_field(&run_id, "tool_running", "tool running", &format!("{} — {}", name, target), iterations);
            let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                DataObject::from_string(&args_str)
            }));
            match parsed {
                Err(_) => format!("Error: tool arguments were not a valid JSON object: {}", &args_str),
                Ok(arguments) => {
                    let parts: Vec<&str> = name.split('-').collect();
                    if parts.len() != 3 {
                        format!("Error: Invalid tool name '{}'. Must be in the format 'library-control-command'", name)
                    } else {
                        let execution_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let cmd = Command::lookup(parts[0], parts[1], parts[2]);
                            cmd.cast_params(arguments.clone());
                            cmd.execute(arguments)
                        }));
                        match execution_result {
                            Ok(Ok(v)) => {
                                // Unwrap the wrapper's {"a": ...} packaging when that's the
                                // whole payload, so the model sees the actual result.
                                match v.try_get_object("a") {
                                    Ok(inner) => inner.to_string(),
                                    _ => v.to_string()
                                }
                            },
                            Ok(Err(e)) => format!("Tool Error: {:?}", e),
                            Err(panic_err) => {
                                let msg = if let Some(s) = panic_err.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_err.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown internal panic".to_string()
                                };
                                // Honest diagnosis: compile errors are compile errors,
                                // not "check your formatting".
                                if msg.contains("error[") || msg.trim_start().starts_with("error") {
                                    format!("Compilation failed. Fix the code and try again:\n{}", msg)
                                } else {
                                    format!("Tool panicked: '{}'. This is a runtime failure in the tool, not a formatting problem. Reconsider the arguments or approach before retrying.", msg)
                                }
                            }
                        }
                    }
                }
            }
        };

        // Dirty-tracking for the verify nudge
        if name.ends_with("upsert_command") || name.ends_with("patch_command_body") {
            nudged = false;
            dirty = name.clone();
            if let Ok(a) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| DataObject::from_string(&args_str))) {
                if a.has("lib") && a.has("ctl") && a.has("cmd") {
                    dirty = format!("{}-{}-{}", a.get_string("lib"), a.get_string("ctl"), a.get_string("cmd"));
                }
            }
        }
        if name.ends_with("invoke_command") || name.ends_with("evaluate_rust") || name.ends_with("delete_command") {
            dirty = String::new();
            nudged = false;
        }

        // Budget the observation: rustc-bearing tools get error-extraction @ 8KB, others 6KB
        let obs = if name.ends_with("upsert_command") || name.ends_with("patch_command_body") {
            clamp(&extract_compile_errors(&observation), 8000)
        } else {
            clamp(&observation, 6000)
        };
      
        let obs_head: String = obs.chars().take(300).collect();
        println!("TL[{}] <- {} [{}B] {}", iterations, name, obs.len(), obs_head.replace('\n', " | "));

        let mut toolmsg = DataObject::new();
        toolmsg.put_string("role", "tool");
        toolmsg.put_string("tool_call_id", &call_id);
        toolmsg.put_string("content", &obs);
        messages.push_object(toolmsg);
      
        let is_failure = obs.contains("was not registered")
            || obs.contains("\"status\":\"err\"")
            || obs.starts_with("error")
            || obs.starts_with("Tool Error")
            || obs.starts_with("Tool panicked")
            || obs.starts_with("Compilation failed")
            || obs.contains("You already called this exact tool");
        if !duplicate {
            last_action = format!("{} — {}", name, target);
            last_observation = obs.clone();
            if !run_id.is_empty() {
                let observed = clamp(&format!("IN-FLIGHT OBSERVATION (unfinished; not verified completion)\nROUND: {}\nLAST ACTION: {}\nLAST RESULT:\n{}\nNEXT: Reconstruct from the observed action and result; inspect this run's activity. Do not repeat an identical failed call without new information.", iterations, last_action, last_observation), 6000);
                let mut ce = DataObject::new(); ce.put_string("kind", "observation");
                ce.put_string("checkpoint", &observed);
                let mut h = DataObject::new(); h.put_string("checkpoint", &observed);
                h.put_string("next", "Recover cautiously from this unfinished observation; no model-authored handoff was reached.");
                h.put_int("round", iterations); h.put_int("max_rounds", max_iterations);
                h.put_int("tool_calls", executed_calls); h.put_int("errors", error_count);
                ce.put_object("handoff", h); activity_emit(&run_id, ce);
            }
        }
        if is_failure {
            error_count += 1;
            let err_head: String = obs.chars().take(120).collect();
            if err_head == last_err_head {
                consecutive_failures += 1;
            } else {
                consecutive_failures = 1;
                last_err_head = err_head;
            }
        } else {
            consecutive_failures = 0;
            last_err_head = String::new();
        }
        {
            let mut e = DataObject::new(); e.put_string("kind", "tool_result");
            e.put_string("phase", if is_failure { "tool failed" } else { "tool succeeded" });
            e.put_string("state", if is_failure { "tool failed" } else { "tool succeeded" });
            e.put_string("tool", &name); e.put_string("target", &target);
            e.put_boolean("ok", !is_failure && !duplicate);
            e.put_int("round", iterations); e.put_int("tool_calls", executed_calls);
            e.put_int("errors", error_count); e.put_int("failure_streak", consecutive_failures);
            e.put_int("duplicate_streak", repeat_count); e.put_int("streak", consecutive_failures);
            e.put_string("detail", &clamp(&obs, 4000)); activity_emit(&run_id, e);
        }
        if consecutive_failures >= 3 {
            final_answer = format!("Aborting: the same failure occurred {} times in a row from {} with no change in outcome: {}", consecutive_failures, name, obs.chars().take(500).collect::<String>());
            checkpoint_reason = "Consecutive identical failure guard stopped this run.".to_string();
            break;
        }
    }
    if !final_answer.is_empty() {
        if checkpoint_reason.is_empty() && (final_answer.starts_with("Aborting:") || final_answer.starts_with("Error:") || final_answer.starts_with("LLM backend error:")) {
            checkpoint_reason = "The run stopped before a verified completion.".to_string();
        }
        break;
    }
}

// the archivist's intake (docs/memory.md): a completed loop is a turn.
// Panic-shielded fire-and-forget - logging must never cost the answer.
let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let logcmd = Command::lookup("agent", "archivist", "log_turn");
    let mut la = DataObject::new();
    la.put_string("venue", "tool_loop");
    la.put_string("ask", &prompt.chars().take(4000).collect::<String>());
    la.put_string("reply", &final_answer.chars().take(4000).collect::<String>());
    la.put_string("tools", "");
    la.put_string("author", "tool_loop");
    let _ = logcmd.execute(la);
}));

// Parse the model's completion status out of the final answer's STATUS: marker
// (the executive prompt instructs DONE/IN_PROGRESS/BLOCKED). This is the signal
// step_item uses to close the loop: DONE -> verify + completed, BLOCKED
// -> the item moves to the `blocked` lifecycle (the plan loop skips it until a
// human re-accepts it - never a silent re-prove). Default: a budget checkpoint
// is incomplete; a clean text finish with no marker is IN_PROGRESS (the item
// stays accepted and keeps being stepped).
let mut status = "IN_PROGRESS".to_string();
if final_answer.starts_with("[budget checkpoint") {
    status = "IN_PROGRESS".to_string();
}
let has_evidence = final_answer.contains("VERIFIED:");
let mut declared_done = false;
for line in final_answer.lines() {
    let t = line.trim();
    if let Some(rest) = t.strip_prefix("STATUS:") {
        let v = rest.trim().to_uppercase();
        // DONE requires a VERIFIED: evidence line (the executive prompt's
        // contract); a bare DONE claim with no check actually run is treated
        // as IN_PROGRESS so an unverified claim can't prematurely close an item.
        if v.starts_with("DONE") && has_evidence { declared_done = true; }
        else if v.starts_with("BLOCKED") { status = "BLOCKED".to_string(); }
        else if v.starts_with("IN_PROGRESS") { status = "IN_PROGRESS".to_string(); }
        break;
    }
}
// GOAL-SCOPE GATE (owner, 2026-10): a VERIFIED: line only proves the model ran
// *a* check - not that the check satisfies the item's GOAL. The loop's observed
// failure mode is premature closure on a PHASE-scoped increment ("P3 complete
// and verified") accepted as the whole multi-phase GOAL. Before accepting DONE,
// grade the evidence against the goal with ONE bounded model call. FAIL CLOSED:
// any grader error, non-SATISFIED answer, or empty goal parses as IN_PROGRESS,
// never DONE - an ambiguous completion can always be re-stepped, a false DONE
// silently ends the loop. Skipped entirely unless a DONE is even claimed, so
// IN_PROGRESS/BLOCKED turns never pay for it.
if declared_done {
    let goal = {
        // the item's claim rides the user prompt as "GOAL: <claim>"; recover it
        // so the grader judges against the goal, not the increment.
        let mut g = String::new();
        if let Some(i) = prompt.find("GOAL:") {
            let rest = &prompt[i + 5..];
            let end = rest.find("\n\n").unwrap_or(rest.len());
            g = rest[..end].trim().to_string();
        }
        if g.is_empty() { g = prompt.chars().take(600).collect(); }
        g
    };
    let verdict = {
        let gprompt = format!(
            "You are a strict completion gate for an autonomous work item.\n\
             GOAL (the whole thing that must be true to call the item done):\n{}\n\n\
             The worker declared DONE and gave this completion report and VERIFIED evidence:\n{}\n\n\
             Decide ONLY whether the report's verified evidence demonstrates the WHOLE GOAL is satisfied - \
             not merely that one phase, increment, or sub-part of it is done. \
             A report that completes P3 of a P0-P4 plan, or builds a parallel artifact without changing the \
             thing the goal names, or verifies a spike/gate rather than the end state, is NOT satisfied.\n\
             Answer with exactly one word on the first line: SATISFIED or PARTIAL.",
            goal, final_answer.chars().take(2500).collect::<String>());
        // ask_llm returns FLAT: the text lands under "a" ({a:"SATISFIED"}), so
        // pull it back out - matching on the raw JSON would never see the word.
        let raw = crate::agent::llm::ask_llm::ask_llm(gprompt, ndata::data::Data::DString(
            "You are a strict completion gate. Answer with exactly one word: SATISFIED or PARTIAL.".to_string()));
        raw
    };
    // Normalize: strip the FLAT {"a":"..."} wrapper if present, then read the word.
    let v = {
        let t = verdict.trim();
        let inner = if t.starts_with('{') {
            // {"a":"SATISFIED"} -> SATISFIED ; fall back to the raw text otherwise
            match ndata::dataobject::DataObject::try_from_string(t) {
                Ok(o) => o.try_get_string("a").unwrap_or_else(|_| t.to_string()),
                Err(_) => t.to_string(),
            }
        } else { t.to_string() };
        inner.trim().trim_matches('"').to_uppercase()
    };
    if v.starts_with("SATISFIED") {
        status = "DONE".to_string();
    } else {
        // PARTIAL, ERROR:, or anything else -> keep the loop going.
        status = "IN_PROGRESS".to_string();
    }
}

if !run_id.is_empty() {
    let checkpoint = if !pending_checkpoint.is_empty() { pending_checkpoint.clone() } else {
        // If the model left an explicit NEXT, retain it verbatim instead of
        // replacing it with a generic instruction. Always append observed
        // action/result facts so an unfinished response is still resumable.
        clamp(&format!("CONTINUATION FROM PREVIOUS PLAN TURN\nSTATUS: {}\nRESPONSE / EVIDENCE:\n{}\nLAST ACTION: {}\nLAST RESULT:\n{}\nNEXT: Resume from the response's NEXT/action when present; otherwise inspect run activity and choose a new concrete increment. Do not repeat an identical failed call without new information.",
            status, final_answer, if last_action.is_empty() { "none".to_string() } else { last_action.clone() },
            if last_observation.is_empty() { "none".to_string() } else { last_observation.clone() }), 8000)
    };
    let mut ce = DataObject::new(); ce.put_string("kind", "checkpoint");
    ce.put_string("phase", "checkpoint saved"); ce.put_string("state", "checkpoint saved");
    ce.put_string("detail", &checkpoint_reason); ce.put_string("checkpoint", &checkpoint);
    let mut h = DataObject::new(); h.put_string("checkpoint", &checkpoint);
    h.put_string("next", "Resume from this checkpoint; inspect this run's activity if more detail is needed.");
    h.put_int("round", iterations); h.put_int("max_rounds", max_iterations);
    h.put_int("tool_calls", executed_calls); h.put_int("errors", error_count);
    h.put_string("recovery", "Use the checkpoint and this run's safe activity timeline. Raw tool arguments and serialized provider transcripts are not retained here.");
    ce.put_object("handoff", h); activity_emit(&run_id, ce);
    let mut e = DataObject::new(); e.put_string("kind", "finish");
    e.put_string("phase", "finished"); e.put_string("state", "run finished");
    e.put_string("outcome", &status); e.put_string("detail", &final_answer);
    e.put_int("round", iterations); e.put_int("max_rounds", max_iterations);
    e.put_int("tool_calls", executed_calls); e.put_int("errors", error_count);
    e.put_int("failure_streak", consecutive_failures); e.put_int("duplicate_streak", repeat_count);
    activity_emit(&run_id, e);
}
let mut response = DataObject::new();
if !run_id.is_empty() { response.put_string("run_id", &run_id); }
response.put_string("msg", &final_answer);
response.put_string("status", &status);
response.put_array("messages", messages);
response