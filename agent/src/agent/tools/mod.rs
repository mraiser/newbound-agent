// This file is auto-generated and managed by the flowlang build script.
use flowlang::rustcmd::Transform;
pub mod rsync_push;
pub mod ssh_run;
pub fn cmdinit(cmds: &mut Vec<(String, Transform, String)>) {
    cmds.push(("lqmggg1a038e57681y2".to_string(), ssh_run::execute, "".to_string()));
    cmds.push(("mwqqpm1a038e5b92dn4".to_string(), rsync_push::execute, "".to_string()));
}
