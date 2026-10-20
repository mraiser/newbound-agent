// This file is auto-generated and managed by the flowlang build script.
use flowlang::rustcmd::Transform;
pub mod step_item;
pub mod add_item;
pub mod move_item;
pub mod board;
pub fn cmdinit(cmds: &mut Vec<(String, Transform, String)>) {
    cmds.push(("hgqxqv1a03a50c89cw8".to_string(), board::execute, "".to_string()));
    cmds.push(("ijyjoz1a03a510268ta".to_string(), move_item::execute, "".to_string()));
    cmds.push(("uyunpg1a03a5137c3yc".to_string(), add_item::execute, "".to_string()));
    cmds.push(("opjijl1a158e0142bq1bd6".to_string(), step_item::execute, "".to_string()));
}
