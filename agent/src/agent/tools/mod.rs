// This file is auto-generated and managed by the flowlang build script.
use flowlang::rustcmd::Transform;
pub mod run_local;
pub mod get_path_info;
pub mod write_properties;
pub mod read_properties;
pub mod list_dir;
pub mod read_file;
pub mod get_time;
pub mod rsync_push;
pub mod ssh_run;
pub fn cmdinit(cmds: &mut Vec<(String, Transform, String)>) {
    cmds.push(("lqmggg1a038e57681y2".to_string(), ssh_run::execute, "".to_string()));
    cmds.push(("mwqqpm1a038e5b92dn4".to_string(), rsync_push::execute, "".to_string()));
    cmds.push(("yjtzkw1a0ed711c7eq1".to_string(), get_time::execute, "".to_string()));
    cmds.push(("zimtvy1a0ed7164adw1".to_string(), read_file::execute, "".to_string()));
    cmds.push(("oxpnry1a0ed7192e6u1".to_string(), list_dir::execute, "".to_string()));
    cmds.push(("ymgzvi1a0ed71c361w1".to_string(), read_properties::execute, "".to_string()));
    cmds.push(("lwviqs1a0ed71cd59j1".to_string(), write_properties::execute, "".to_string()));
    cmds.push(("nqojhg1a0ed71d6e3r1".to_string(), get_path_info::execute, "".to_string()));
    cmds.push(("nsuqxx1a0ed71e06bk1".to_string(), run_local::execute, "".to_string()));
}
