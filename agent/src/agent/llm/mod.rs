// This file is auto-generated and managed by the flowlang build script.
use flowlang::rustcmd::Transform;
pub mod last_request;
pub mod llm_remote;
pub mod llm_local;
pub mod llm_ollama;
pub mod llm_gemini;
pub mod llm_anthropic;
pub mod llm_openai;
pub mod llm_common;
pub mod claude_code;
pub mod chat_llm;
pub mod tool_loop;
pub mod ask_llm;
pub fn cmdinit(cmds: &mut Vec<(String, Transform, String)>) {
    cmds.push(("rjuoqv19e8fc5c83ft4".to_string(), ask_llm::execute, "".to_string()));
    cmds.push(("lnmvtl19edbeb72a7tc3a".to_string(), tool_loop::execute, "".to_string()));
    cmds.push(("ytohmk19f70b2c09ck7ce2".to_string(), chat_llm::execute, "".to_string()));
    cmds.push(("mqghlt1a00a71c647q1".to_string(), claude_code::execute, "".to_string()));
    cmds.push(("yjtwjo1a0f76c075aw168".to_string(), llm_common::execute, "".to_string()));
    cmds.push(("kyrpov1a0f76c0e7cj16a".to_string(), llm_openai::execute, "".to_string()));
    cmds.push(("ltmonk1a0f76c15dag16c".to_string(), llm_anthropic::execute, "".to_string()));
    cmds.push(("rosqph1a0f76c1d5fp16f".to_string(), llm_gemini::execute, "".to_string()));
    cmds.push(("gmjnvy1a0f76c2535s171".to_string(), llm_ollama::execute, "".to_string()));
    cmds.push(("prnmjj1a0f76c2d46r173".to_string(), llm_local::execute, "".to_string()));
    cmds.push(("pktyxy1a0f76c2dddj175".to_string(), llm_remote::execute, "".to_string()));
    cmds.push(("hnnxvh1a0f8ea39d3g117".to_string(), last_request::execute, "".to_string()));
}
