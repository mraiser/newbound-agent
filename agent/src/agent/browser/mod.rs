// This file is auto-generated and managed by the flowlang build script.
use flowlang::rustcmd::Transform;
pub mod screenshot;
pub mod close;
pub mod wait_for;
pub mod fill;
pub mod click;
pub mod text;
pub mod goto;
pub mod open;
pub mod eval;
pub fn cmdinit(cmds: &mut Vec<(String, Transform, String)>) {
    cmds.push(("xhuqpr1a03b7fe957i8".to_string(), eval::execute, "".to_string()));
    cmds.push(("jqgspz1a03b805a1bja".to_string(), open::execute, "".to_string()));
    cmds.push(("tzwzqk1a03b80b9a5zc".to_string(), goto::execute, "".to_string()));
    cmds.push(("lxgqyp1a03b80e4c0te".to_string(), text::execute, "".to_string()));
    cmds.push(("igmtmw1a03b810adfk10".to_string(), click::execute, "".to_string()));
    cmds.push(("nhvyyr1a03b817969g14".to_string(), fill::execute, "".to_string()));
    cmds.push(("xkiujg1a03b821c3fu16".to_string(), wait_for::execute, "".to_string()));
    cmds.push(("ykmzmg1a03b82db11g1e".to_string(), close::execute, "".to_string()));
    cmds.push(("rvuzgy1a03fb16529n1".to_string(), screenshot::execute, "".to_string()));
}
