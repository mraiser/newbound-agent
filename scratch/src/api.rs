#![allow(non_camel_case_types, unused_variables, unused_imports, dead_code)]
pub use ::ndata::dataobject::DataObject;
pub use ::ndata::dataarray::DataArray;
pub use ::ndata::databytes::DataBytes;
pub use ::ndata::data::Data;

pub mod agent {
    pub mod agent {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod llm {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn ask_llm(prompt: String, system_prompt: Data) -> String {
            let mut d = DataObject::new();
            d.put_string("prompt", &prompt);
            d.set_property("system_prompt", system_prompt);
            ::flowlang::rustcmd::RustCmd::new("rjuoqv19e8fc5c83ft4").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn tool_loop(prompt: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("prompt", &prompt);
            ::flowlang::rustcmd::RustCmd::new("lnmvtl19edbeb72a7tc3a").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn chat_llm(messages: DataArray, tools: DataArray) -> DataObject {
            let mut d = DataObject::new();
            d.put_array("messages", messages);
            d.put_array("tools", tools);
            ::flowlang::rustcmd::RustCmd::new("ytohmk19f70b2c09ck7ce2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn claude_code(messages: DataArray, tools: DataArray) -> DataObject {
            let mut d = DataObject::new();
            d.put_array("messages", messages);
            d.put_array("tools", tools);
            ::flowlang::rustcmd::RustCmd::new("mqghlt1a00a71c647q1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod plugin {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn control_query(message: String, context: DataObject) -> String {
            let mut d = DataObject::new();
            d.put_string("message", &message);
            d.put_object("context", context);
            ::flowlang::rustcmd::RustCmd::new("innxiu19ebbb8efe6yfdf").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn list_tools() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("sroyxx19ebde8708fk14aa").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn describe_command(command_name: String, lang: String, returntype: String, groups: String, params: DataArray, imports: String, code: String, current_description: String) -> String {
            let mut d = DataObject::new();
            d.put_string("command_name", &command_name);
            d.put_string("lang", &lang);
            d.put_string("returntype", &returntype);
            d.put_string("groups", &groups);
            d.put_array("params", params);
            d.put_string("imports", &imports);
            d.put_string("code", &code);
            d.put_string("current_description", &current_description);
            ::flowlang::rustcmd::RustCmd::new("ktoprh19ec10b7907k1b87").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod scratch {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn eval_pshkms19ee68b2a1ct46() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ukvisj19ee68b2a21o48").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod agentloop {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod agentprompt {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod memory {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod archivist {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn log_turn(venue: String, ask: String, reply: String, tools: String, author: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("venue", &venue);
            d.put_string("ask", &ask);
            d.put_string("reply", &reply);
            d.put_string("tools", &tools);
            d.put_string("author", &author);
            ::flowlang::rustcmd::RustCmd::new("zktsrl19fb904ad42r2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn consolidate() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("lwzzvz19fb904b9f0m4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn queue_status() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("grkhrm19fb91df28dj1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remember(lib: String, domain: String, entry: DataObject, author: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("domain", &domain);
            d.put_object("entry", entry);
            d.put_string("author", &author);
            ::flowlang::rustcmd::RustCmd::new("kkjzwq19fec41bc01j1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn promote(lib: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("ovppsz1a001b4abacu1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn seed_export(domains: String, path: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("domains", &domains);
            d.put_string("path", &path);
            ::flowlang::rustcmd::RustCmd::new("wjrzko1a001b4c938j3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn bootstrap(path: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("path", &path);
            ::flowlang::rustcmd::RustCmd::new("qxinhl1a001b4d45ei5").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recall(query: String, domains: String, limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("query", &query);
            d.put_string("domains", &domains);
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("jwluwr1a0063833d7g1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn adjudicate(lib: String, domain: String, entry: DataObject, author: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("domain", &domain);
            d.put_object("entry", entry);
            d.put_string("author", &author);
            ::flowlang::rustcmd::RustCmd::new("ytjnql1a006791e27h1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn epistemic_work() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ixhqrg1a0068b1a0cx1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn decay(lib: String, domain: String, claim: String, author: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("domain", &domain);
            d.put_string("claim", &claim);
            d.put_string("author", &author);
            ::flowlang::rustcmd::RustCmd::new("mttpgg1a0068b31e0u3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn reverify(limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("xknmpg1a01a30b0bdw1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn connect(subject: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("subject", &subject);
            ::flowlang::rustcmd::RustCmd::new("slxnyn1a01a32718ar1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn wonder() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("thpngr1a01a328e34q1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod chat {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn upload(filename: String, data_b64: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("filename", &filename);
            d.put_string("data_b64", &data_b64);
            ::flowlang::rustcmd::RustCmd::new("sspmvm1a039233859t28").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod askrow {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod describebtn {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod prompts {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod executive {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn start() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("qosmvt1a005283299g2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn stop() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ivhzuq1a005289448q4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn status() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("posxgg1a005289fd4u6").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn perceive(perception: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("perception", perception);
            ::flowlang::rustcmd::RustCmd::new("rstxhp1a00528ab29h8").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_drive(acts_per_hour: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_int("acts_per_hour", acts_per_hour);
            ::flowlang::rustcmd::RustCmd::new("yhmiqo1a0068b5d24m5").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn salience_log() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("pqphsl1a0069ec4b0j1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn consolidate_room(min_quiet_s: i64, window: i64, budget: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_int("min_quiet_s", min_quiet_s);
            d.put_int("window", window);
            d.put_int("budget", budget);
            ::flowlang::rustcmd::RustCmd::new("qimijq1a01a19edbdl1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod sensor {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn start() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("spjnyl1a00643ca37w2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn stop() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mlmloh1a00643d901g4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn status() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("shvpqu1a00643e6b2p6").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn system_sense() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("huqsxm1a01a171a3fv1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn git_sense() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("tlurvg1a0245e69c0t1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod model {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn salience(perception: DataObject, context: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("perception", perception);
            d.put_object("context", context);
            ::flowlang::rustcmd::RustCmd::new("gkrolu1a007e29aaeq2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn service_status() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("smkzti1a007e309a2z4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn curriculum_export(path: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("path", &path);
            ::flowlang::rustcmd::RustCmd::new("uvwngs1a007e317dfx6").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn bootstrap() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mmgqil1a007f2ef9dz1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn train_status() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("qmlyql1a00b6e9588w1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get_settings() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("pvstyk1a00b6edc7fo3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_setting(key: String, value: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("key", &key);
            d.put_string("value", &value);
            ::flowlang::rustcmd::RustCmd::new("snntws1a00b6eeefbp5").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn promote_pointer() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("rvkipx1a00b6f02c0y7").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn metrics() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("xwjiht1a00b8b4d59o1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn service_stop() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("tqqiiv1a00f530e92n1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn user_promote() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("pigtxk1a01099f7fby1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn user_rollback() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("hjuwvn1a0109a1288u3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn persona_rederive() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("wwhrxv1a01124a788x1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn persona_read() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("uwlztr1a0113e5c0fn1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn persona_write(content: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("content", &content);
            ::flowlang::rustcmd::RustCmd::new("pyxiiz1a0113e7d1eu3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn import(name: String, source: String, backend: String, anchor: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("source", &source);
            d.put_string("backend", &backend);
            d.put_string("anchor", &anchor);
            ::flowlang::rustcmd::RustCmd::new("zszomo1a017741685j1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn models() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("yjmwrj1a0177498a8v1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn model_remove(name: String, purge: bool) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_boolean("purge", purge);
            ::flowlang::rustcmd::RustCmd::new("ggojst1a01774ad4dg1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn resources() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("qzikks1a01776d049o1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_add(name: String, source: String, kind: String, format: String, holdout_every: i64, mode: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("source", &source);
            d.put_string("kind", &kind);
            d.put_string("format", &format);
            d.put_int("holdout_every", holdout_every);
            d.put_string("mode", &mode);
            ::flowlang::rustcmd::RustCmd::new("srzlok1a0178434d1k1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_list() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("iklvto1a017845e1ar1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_inspect(name: String, peek: i64, verify: bool) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_int("peek", peek);
            d.put_boolean("verify", verify);
            ::flowlang::rustcmd::RustCmd::new("uvqhnt1a0178473cbm1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_snapshot(name: String, snapshot_name: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("snapshot_name", &snapshot_name);
            ::flowlang::rustcmd::RustCmd::new("jimxoz1a0178489a2h1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_derive(name: String, out_name: String, transform: String, limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("out_name", &out_name);
            d.put_string("transform", &transform);
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("oopiwl1a01784a023z1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_remove(name: String, purge: bool) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_boolean("purge", purge);
            ::flowlang::rustcmd::RustCmd::new("rvjoug1a01787534du1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn adapter_derive(name: String, dataset: String, base: String, targets: String, rank: i64, steps: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("dataset", &dataset);
            d.put_string("base", &base);
            d.put_string("targets", &targets);
            d.put_int("rank", rank);
            d.put_int("steps", steps);
            ::flowlang::rustcmd::RustCmd::new("uglrzs1a017b4932dm1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn adapter_apply(name: String, on: bool) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_boolean("on", on);
            ::flowlang::rustcmd::RustCmd::new("rzrtvh1a017b4aa9ez1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn adapters() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("zwhrqp1a017b4c175s1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn adapter_delete(name: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            ::flowlang::rustcmd::RustCmd::new("rpszoz1a017b4d8cez1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recipe_author(name: String, base: String, mix: String, posture: String, steps: i64, lr: String, evals: String, notes: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("base", &base);
            d.put_string("mix", &mix);
            d.put_string("posture", &posture);
            d.put_int("steps", steps);
            d.put_string("lr", &lr);
            d.put_string("evals", &evals);
            d.put_string("notes", &notes);
            ::flowlang::rustcmd::RustCmd::new("goumpq1a01926744cg1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recipe_clone(name: String, from: String, edits: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("from", &from);
            d.put_object("edits", edits);
            ::flowlang::rustcmd::RustCmd::new("jnhhkw1a01926c03eu1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recipes() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("nonruq1a01926d805k1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recipe_remove(name: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            ::flowlang::rustcmd::RustCmd::new("wyhpqs1a01926eea0z1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn experiment(name: String, control: String, variant: String, budget_steps: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("control", &control);
            d.put_string("variant", &variant);
            d.put_int("budget_steps", budget_steps);
            ::flowlang::rustcmd::RustCmd::new("rzkmjv1a01927050dt1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn experiments() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("jypzzo1a019271a79o1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn sft_run(name: String, dataset: String, base: String, rank: i64, steps: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("dataset", &dataset);
            d.put_string("base", &base);
            d.put_int("rank", rank);
            d.put_int("steps", steps);
            ::flowlang::rustcmd::RustCmd::new("uoioiv1a01938479ci1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn sft_promote(checkpoint: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("checkpoint", &checkpoint);
            ::flowlang::rustcmd::RustCmd::new("zzzosn1a019385f6fn1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn dataset_feed(name: String, kind: String, lines: String, lineage: String, provenance: String, holdout_every: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("kind", &kind);
            d.put_string("lines", &lines);
            d.put_string("lineage", &lineage);
            d.put_string("provenance", &provenance);
            d.put_int("holdout_every", holdout_every);
            ::flowlang::rustcmd::RustCmd::new("vvhumz1a019d18b18t1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn why_harvest(source: String, repo_path: String, limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("source", &source);
            d.put_string("repo_path", &repo_path);
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("gnvkzr1a01a0b0e05g1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn harvest_report(window_days: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_int("window_days", window_days);
            ::flowlang::rustcmd::RustCmd::new("hgkzok1a01a3db91ck1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod msg {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn put(role: String, venue: String, content: String, entity: String, provenance: String, id: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("role", &role);
            d.put_string("venue", &venue);
            d.put_string("content", &content);
            d.put_string("entity", &entity);
            d.put_string("provenance", &provenance);
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("mhtnxo1a019c47805n1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get(id: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("qhtrpu1a019c59f5ep1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn recent(venue: String, limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("venue", &venue);
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("qvoxjm1a019c5b988h1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod context {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn assemble(purpose: String, subject: String, budget: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("purpose", &purpose);
            d.put_string("subject", &subject);
            d.put_int("budget", budget);
            ::flowlang::rustcmd::RustCmd::new("qszxrr1a019d94db9r1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod tools {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn ssh_run(host: String, cmd: String, timeout_secs: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("host", &host);
            d.put_string("cmd", &cmd);
            d.put_int("timeout_secs", timeout_secs);
            ::flowlang::rustcmd::RustCmd::new("lqmggg1a038e57681y2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn rsync_push(host: String, src: String, dst: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("host", &host);
            d.put_string("src", &src);
            d.put_string("dst", &dst);
            ::flowlang::rustcmd::RustCmd::new("mwqqpm1a038e5b92dn4").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
}

pub mod app {
    pub mod api {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod app {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn apps() -> DataArray {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ynjjnl182f0c30c2ej26bb").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn asset(nn_path: String) -> String {
            let mut d = DataObject::new();
            d.put_string("nn_path", &nn_path);
            ::flowlang::rustcmd::RustCmd::new("hxusrn182ebab0fc8o1102").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn assets(lib: String) -> DataArray {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("uirppm183059f5a37z1b0c").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn delete(lib: String, id: String, nn_sessionid: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("id", &id);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("mhnrjq18347bcd5f7t27").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn deletelib(lib: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("hkgorn1834268eb07k1406").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn deviceid() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("jypyqw1836795f8fbn2").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn eventoff(id: String) -> String {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("xrysgt18350cb35cet3").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn eventon(id: String, app: String, event: String, cmdlib: String, cmdid: String) -> String {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            d.put_string("app", &app);
            d.put_string("event", &event);
            d.put_string("cmdlib", &cmdlib);
            d.put_string("cmdid", &cmdid);
            ::flowlang::rustcmd::RustCmd::new("wlnoru18350ecc36cr4").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn events(app: String) -> DataArray {
            let mut d = DataObject::new();
            d.put_string("app", &app);
            ::flowlang::rustcmd::RustCmd::new("spumvi1834c2cf1e6t2").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn exec(lib: String, id: String, args: DataObject, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("id", &id);
            d.put_object("args", args);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("thoxjp182ee8eaebdt225").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn jsapi(nn_path: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("nn_path", &nn_path);
            ::flowlang::rustcmd::RustCmd::new("zmzwjn182ee9c7f0ar314").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn libs() -> DataArray {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("vtnluk1834262fb3fl137e").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn login(user: String, pass: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("user", &user);
            d.put_string("pass", &pass);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ztizvj182ee99186cp2d2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn newlib(lib: String, readers: DataArray, writers: DataArray) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_array("readers", readers);
            d.put_array("writers", writers);
            ::flowlang::rustcmd::RustCmd::new("stskpj183421d8115xd3f").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn read(lib: String, id: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("id", &id);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("nyzimq182eabf7339p7c5").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remembersession(nn_session: DataObject) -> String {
            let mut d = DataObject::new();
            d.put_object("nn_session", nn_session);
            ::flowlang::rustcmd::RustCmd::new("tsmxsj182ee9ac271o2f3").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn settings(settings: Data) -> DataObject {
            let mut d = DataObject::new();
            d.set_property("settings", settings);
            ::flowlang::rustcmd::RustCmd::new("knhvsn182f9997b1dxd04").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn spawn(lib: String, ctl: String, cmd: String, args: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_object("args", args);
            ::flowlang::rustcmd::RustCmd::new("tvigvw19268109f0fg2a60").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn timeroff(id: String) -> String {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("hompli1835678a4efz2").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn timeron(id: String, data: DataObject) -> String {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            d.put_object("data", data);
            ::flowlang::rustcmd::RustCmd::new("spjvvp183568021f1o2").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn uninstall(app: String) -> String {
            let mut d = DataObject::new();
            d.put_string("app", &app);
            ::flowlang::rustcmd::RustCmd::new("gttrqg18303bc96c9w898").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn unique_session_id() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ynpmir183479da2b9r25f8").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn write(lib: String, id: Data, data: DataObject, readers: Data, writers: Data, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.set_property("id", id);
            d.put_object("data", data);
            d.set_property("readers", readers);
            d.set_property("writers", writers);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("yjjxqk18303e75f8atb5a").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod appcard {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod appinfo {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod dial {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod list {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod list_item {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod login {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod scenegraph {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod select {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod service {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn init() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mjkrmm183e1fdb2d2r8").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod shape {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod ui {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod ui_reference {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod util {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn hash(file: String) -> String {
            let mut d = DataObject::new();
            d.put_string("file", &file);
            ::flowlang::rustcmd::RustCmd::new("kgkxpw183664f5554q4").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn init() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("thtpku18366290644p4").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn zip(srcdir: String, destfile: String) -> bool {
            let mut d = DataObject::new();
            d.put_string("srcdir", &srcdir);
            d.put_string("destfile", &destfile);
            ::flowlang::rustcmd::RustCmd::new("guuqrj1836147b650zd").execute(d).expect("Rust command execution failed").get_boolean("a")
        }

    }
    pub mod sceneplayer {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod sceneexpr {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod scenetokens {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod scenedoc {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod sceneproject {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod scenerun {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod forcelayout {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod tokens {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod webgl {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
}

pub mod dev {
    pub mod dev {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn check(lib: String, ctl: String, cmd: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("gsxkwg184e3fc96f9s2e1").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn compile(lib: String, ctl: String, cmd: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("gjssly1834862d5acg37d9").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn compile_rust() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mhxogz1858786d9e1scf").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn install_lib(uuid: String, lib: String) -> bool {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("kqgjmx1840a9081cdh172").execute(d).expect("Rust command execution failed").get_boolean("a")
        }

        pub fn lib_archive(lib: String, version: i64) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_int("version", version);
            ::flowlang::rustcmd::RustCmd::new("uykmrm183dbd15cdeu7b").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn lib_info(lib: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("knwozu1840a764abcu135").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn rebuild_lib(lib: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("yypums1847731c7fap5").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn activate_lib(lib: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("lrgoyo19fe9049accu1").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod editcommand {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn compile_command(lib: String, control_name: String, cmd_name: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("control_name", &control_name);
            d.put_string("cmd_name", &cmd_name);
            ::flowlang::rustcmd::RustCmd::new("wmjmsm19e30a16655r3439").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn delete_command(lib: String, control_id: String, cmd_id: String, nn_sessionid: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("control_id", &control_id);
            d.put_string("cmd_id", &cmd_id);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("zjkntl19e309a2635o3426").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn save_command(lib: String, cmd_id: String, lang: String, code: String, imports: String, returntype: String, params: DataArray, desc: String, groups: String, readers: DataArray, nn_sessionid: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("cmd_id", &cmd_id);
            d.put_string("lang", &lang);
            d.put_string("code", &code);
            d.put_string("imports", &imports);
            d.put_string("returntype", &returntype);
            d.put_array("params", params);
            d.put_string("desc", &desc);
            d.put_string("groups", &groups);
            d.put_array("readers", readers);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("yqvnwh19e30916b04l3410").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn read_command(lib: String, ctl: String, cmd: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("hkhmnw19e55777c46x41").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn lookup_cmd_id(lib: String, ctl: String, cmd: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("vpqniv19e558047aeo59").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod editcontrol {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn add_component(lib: String, control_id: String, component_type: String, name: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("control_id", &control_id);
            d.put_string("component_type", &component_type);
            d.put_string("name", &name);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ywmyvk19e2d0d215ai2c5b").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn appdata(data: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("data", data);
            ::flowlang::rustcmd::RustCmd::new("vsxqui18332a86185i159").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get_control(lib: String, id: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("ljxttx19e2c502d8bg2ab1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get_publish_context(lib: String, control_id: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("control_id", &control_id);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("lhknos19e2d258cb6w2c94").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn lookup_id(lib: String, name: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("name", &name);
            ::flowlang::rustcmd::RustCmd::new("ggkslj19e2c58bb61q2ac8").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn publishapp(data: DataObject) -> DataArray {
            let mut d = DataObject::new();
            d.put_object("data", data);
            ::flowlang::rustcmd::RustCmd::new("iwvgmq1835bb194ffo8").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn save_control(lib: String, id: String, html: String, css: String, js: String, groups: String, desc: String, readers: DataArray, inline_data: DataObject, nn_sessionid: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("id", &id);
            d.put_string("html", &html);
            d.put_string("css", &css);
            d.put_string("js", &js);
            d.put_string("groups", &groups);
            d.put_string("desc", &desc);
            d.put_array("readers", readers);
            d.put_object("inline_data", inline_data);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("uwoygr19e2c6bab55r2af6").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod github {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn import(url: String) -> String {
            let mut d = DataObject::new();
            d.put_string("url", &url);
            ::flowlang::rustcmd::RustCmd::new("nnjgwh189dcdca95fq7c").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn list() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("lovuhn189dc981ebch2f").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn update(lib: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("hioqsq19fe7789bcaj1").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn remove(lib: String, delete_repository: bool) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_boolean("delete_repository", delete_repository);
            ::flowlang::rustcmd::RustCmd::new("lumrkn19fe778ea1bu3").execute(d).expect("Rust command execution failed").get_string("a")
        }

    }
    pub mod libsettings {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn get_library_config(id: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("gxysqz19721b331c9r54").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn save_library_config(data: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("data", data);
            ::flowlang::rustcmd::RustCmd::new("wjhsqs19720f20d2ct8d").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod plugins {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn list_plugins() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("zvmhyt19763d3e070i43").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod workbench {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod sceneeditor {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod floweditor {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod floweditor3d {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod editor {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod preview {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod shelf {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod card {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod jump {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod frame {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod toast {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod session {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod flowdoc {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod flowproject {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod flowprims {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod flowlayout {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod facets {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod code {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn list_commands(lib: String, ctl: String) -> DataArray {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            ::flowlang::rustcmd::RustCmd::new("ypmryt19ec1558019m1c2c").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn list_controls(lib: String) -> DataArray {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("nhpgow19e9ddf15a2k6").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn list_libraries() -> DataArray {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mtjtsw19e9dd5bcefg1de5").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn add_library(lib: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("kqzknr19ec8a3ea32offa").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn add_control(lib: String, ctl: String) -> String {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            ::flowlang::rustcmd::RustCmd::new("lmywwj19ec8a9e2ccm100b").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn upsert_command(lib: String, ctl: String, cmd: String, lang: String, return_type: String, params: DataArray, imports: String, code_body: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("lang", &lang);
            d.put_string("return_type", &return_type);
            d.put_array("params", params);
            d.put_string("imports", &imports);
            d.put_string("code_body", &code_body);
            ::flowlang::rustcmd::RustCmd::new("ovwolr19ec8c38800z1047").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn patch_command_body(lib: String, ctl: String, cmd: String, old_snippet: String, new_snippet: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("old_snippet", &old_snippet);
            d.put_string("new_snippet", &new_snippet);
            ::flowlang::rustcmd::RustCmd::new("slvzur19ed5ad5cc7k2c99").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn read_command(lib: String, ctl: String, cmd: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("krpzxz19ed5b4aed9v2cad").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn delete_command(lib: String, ctl: String, cmd: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("xqjpyg19ed5c0337dy2cca").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn search_commands(lib: String, ctl: String, query: String) -> DataArray {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("query", &query);
            ::flowlang::rustcmd::RustCmd::new("shlglp19ed5d11bf9i2cf3").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn invoke_command(lib: String, ctl: String, cmd: String, args: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_object("args", args);
            ::flowlang::rustcmd::RustCmd::new("hviwtu19ed5dc7dc5x2d10").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn evaluate_rust(imports: String, code: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("imports", &imports);
            d.put_string("code", &code);
            ::flowlang::rustcmd::RustCmd::new("nwnguj19ee5977c28s1527").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn read_control_facet(lib: String, ctl: String, facet: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("facet", &facet);
            ::flowlang::rustcmd::RustCmd::new("vwswvs19f95a61d29j3943").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn patch_control_facet(lib: String, ctl: String, facet: String, old_snippet: String, new_snippet: String, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("facet", &facet);
            d.put_string("old_snippet", &old_snippet);
            d.put_string("new_snippet", &new_snippet);
            d.put_string("base", &base);
            d.put_string("label", &label);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("uyonls19f95a61d2cp3945").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn list_control_patches(lib: String, ctl: String, limit: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_int("limit", limit);
            ::flowlang::rustcmd::RustCmd::new("zkjqpy19f95a61d2eu3947").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_library_meta(lib: String, desc: String, groups: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("desc", &desc);
            d.put_string("groups", &groups);
            ::flowlang::rustcmd::RustCmd::new("ouqqjw19f95a61d2fu3949").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_control_meta(lib: String, ctl: String, desc: String, groups: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("desc", &desc);
            d.put_string("groups", &groups);
            ::flowlang::rustcmd::RustCmd::new("trogig19f95a61d30w394b").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_command_meta(lib: String, ctl: String, cmd: String, desc: String, groups: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("desc", &desc);
            d.put_string("groups", &groups);
            ::flowlang::rustcmd::RustCmd::new("hlsjpo19f95a61d32q394d").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn list_assets(lib: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            ::flowlang::rustcmd::RustCmd::new("iltsxi19f96bdd724l566d").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn write_asset(lib: String, name: String, content: String, tempfile: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("name", &name);
            d.put_string("content", &content);
            d.put_string("tempfile", &tempfile);
            ::flowlang::rustcmd::RustCmd::new("vxxmwl19f96bdd725r566f").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn rename_asset(lib: String, from: String, to: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("from", &from);
            d.put_string("to", &to);
            ::flowlang::rustcmd::RustCmd::new("qosxxk19f96bdd725z5671").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn delete_asset(lib: String, name: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("name", &name);
            ::flowlang::rustcmd::RustCmd::new("qvipjk19f96bdd726s5673").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn read_flow_body(lib: String, ctl: String, cmd: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            ::flowlang::rustcmd::RustCmd::new("nprqom19f9925517fn789d").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn write_flow_body(lib: String, ctl: String, cmd: String, body: DataObject, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_object("body", body);
            d.put_string("base", &base);
            d.put_string("label", &label);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("vksvyz19f99255185j789f").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_timer(lib: String, ctl: String, name: String, cmd: String, start: i64, startunit: String, interval: i64, intervalunit: String, repeat: bool, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("name", &name);
            d.put_string("cmd", &cmd);
            d.put_int("start", start);
            d.put_string("startunit", &startunit);
            d.put_int("interval", interval);
            d.put_string("intervalunit", &intervalunit);
            d.put_boolean("repeat", repeat);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("kxtnil19f99e8b05bj9cfd").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remove_timer(lib: String, ctl: String, name: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("name", &name);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ppjvhg19f99e8b05fv9cff").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_event_handler(lib: String, ctl: String, name: String, bot: String, event: String, cmd: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("name", &name);
            d.put_string("bot", &bot);
            d.put_string("event", &event);
            d.put_string("cmd", &cmd);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("slwolg19f99e8b060l9d01").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remove_event_handler(lib: String, ctl: String, name: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("name", &name);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("lyqmyx19f99e8b061q9d03").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn read_control_scene(lib: String, ctl: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            ::flowlang::rustcmd::RustCmd::new("vtynpg19fa345cd8eyb2ff").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn write_control_scene(lib: String, ctl: String, scene: DataObject, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_object("scene", scene);
            d.put_string("base", &base);
            d.put_string("label", &label);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ltwuws19fa345cd8erb301").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn delete_library(lib: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("juhgqn19faf571a9az1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn delete_control(lib: String, ctl: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("tjhhxj19faf577471h1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn move_control(lib: String, ctl: String, to_lib: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("to_lib", &to_lib);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("pyjtgq19fb05aeb0cm1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_meta_identity(displayname: String, organization: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("displayname", &displayname);
            d.put_string("organization", &organization);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("pxilgw19fb08d4430k1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get_meta_identity() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("lgiozw19fb094fdfau1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn unpublish_app(lib: String, app: String, remove_runtime: bool, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("app", &app);
            d.put_boolean("remove_runtime", remove_runtime);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ijyuys19fb09ff451g1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_plugin(name: String, target_lib: String, target_ctl: String, plugin_lib: String, plugin_ctl: String, selector: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("target_lib", &target_lib);
            d.put_string("target_ctl", &target_ctl);
            d.put_string("plugin_lib", &plugin_lib);
            d.put_string("plugin_ctl", &plugin_ctl);
            d.put_string("selector", &selector);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("owxtlg19fb3b6cfd6v1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remove_plugin(name: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("znpnwu19fb3b71711u3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_tags(lib: String, ctl: String, cmd: String, tags: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("tags", &tags);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("vsxqpy19fb84a1ba4m1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_groups(lib: String, ctl: String, cmd: String, groups: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("groups", &groups);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ywwgiq19fb84a4e57h3").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_command_imports(lib: String, ctl: String, cmd: String, imports: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("lib", &lib);
            d.put_string("ctl", &ctl);
            d.put_string("cmd", &cmd);
            d.put_string("imports", &imports);
            ::flowlang::rustcmd::RustCmd::new("opoush19fbdfbfefbn1").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn init() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ioyipx19feee23f1bq1").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod viewctx {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod git {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn gitrun(repo: String, verb: String, args: DataArray, mode: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("repo", &repo);
            d.put_string("verb", &verb);
            d.put_array("args", args);
            d.put_string("mode", &mode);
            ::flowlang::rustcmd::RustCmd::new("nqypsj1a02428c568n8").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn read(repo: String, verb: String, args: DataArray) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("repo", &repo);
            d.put_string("verb", &verb);
            d.put_array("args", args);
            ::flowlang::rustcmd::RustCmd::new("gnwoym1a02428f099ra").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn write(repo: String, verb: String, args: DataArray) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("repo", &repo);
            d.put_string("verb", &verb);
            d.put_array("args", args);
            ::flowlang::rustcmd::RustCmd::new("qxjlkg1a024290af0wc").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remote_op(repo: String, verb: String, args: DataArray) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("repo", &repo);
            d.put_string("verb", &verb);
            d.put_array("args", args);
            ::flowlang::rustcmd::RustCmd::new("kjjhrz1a0242924e2ke").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn set_repo(name: String, path: String, origin: String, role: String, autocommit: bool, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("path", &path);
            d.put_string("origin", &origin);
            d.put_string("role", &role);
            d.put_boolean("autocommit", autocommit);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ztnxkn1a0242976cdh10").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn remove_repo(name: String, author: String, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("name", &name);
            d.put_string("author", &author);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ohzpil1a02429afb3x12").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn repos() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("qnmlxy1a02429d988k14").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn autocommit_sweep() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("xlqhrg1a02521633dv7").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
}

pub mod kb {
    pub mod platform_api {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod workflow {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod frontend {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod m2026_07 {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod doctrine {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod nebula {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod camera {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
}

pub mod peer {
    pub mod headsup {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod peer {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn discovery() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("mlrhvx183e6eabd19xb4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn info(nn_sessionid: String, uuid: Data, salt: Data) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("nn_sessionid", &nn_sessionid);
            d.set_property("uuid", uuid);
            d.set_property("salt", salt);
            ::flowlang::rustcmd::RustCmd::new("tkwkml18390d46728m8").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn local(request: DataObject, nn_session: DataObject, nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("request", request);
            d.put_object("nn_session", nn_session);
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("nylhvq183f6b61e43oc2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn peers() -> DataArray {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ywokvt1838c110d92l8").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn remote(nn_path: String, nn_params: DataObject, nn_headers: DataObject) -> DataBytes {
            let mut d = DataObject::new();
            d.put_string("nn_path", &nn_path);
            d.put_object("nn_params", nn_params);
            d.put_object("nn_headers", nn_headers);
            ::flowlang::rustcmd::RustCmd::new("txnvil183f6ffdf58w1d").execute(d).expect("Rust command execution failed").get_bytes("a")
        }

    }
    pub mod peer_model {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
    pub mod reboot {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn init() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("hygrki1842eac55a9w2a").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn reboot() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("jmhvzv1843439faa0i305").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod service {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn close_stream(uuid: String, streamid: i64, write: bool) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_int("streamid", streamid);
            d.put_boolean("write", write);
            ::flowlang::rustcmd::RustCmd::new("zqxtsm18d3d4ef2b3j101").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn discovery() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("vtxmqr183e5ff3ef5u82").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn exec(uuid: String, app: String, cmd: String, params: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_string("app", &app);
            d.put_string("cmd", &cmd);
            d.put_object("params", params);
            ::flowlang::rustcmd::RustCmd::new("nmojwg18386b2f0d2n2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn get_stream(uuid: String, stream_id: i64) -> DataBytes {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_int("stream_id", stream_id);
            ::flowlang::rustcmd::RustCmd::new("hlmugl188ab38379arb5").execute(d).expect("Rust command execution failed").get_bytes("a")
        }

        pub fn init() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("grvupm18379e9a159n8").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn listen(ipaddr: String, port: i64) -> i64 {
            let mut d = DataObject::new();
            d.put_string("ipaddr", &ipaddr);
            d.put_int("port", port);
            ::flowlang::rustcmd::RustCmd::new("irxuhn18379cef5bcp4").execute(d).expect("Rust command execution failed").get_int("a")
        }

        pub fn listen_udp(ipaddr: String, port: i64) -> i64 {
            let mut d = DataObject::new();
            d.put_string("ipaddr", &ipaddr);
            d.put_int("port", port);
            ::flowlang::rustcmd::RustCmd::new("rgxowg183ad6b7a12u6").execute(d).expect("Rust command execution failed").get_int("a")
        }

        pub fn maintenance() -> String {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("rjntml18385b15b5ch0").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn new_stream(uuid: String) -> i64 {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            ::flowlang::rustcmd::RustCmd::new("myuvuz18d36f76d2cg3").execute(d).expect("Rust command execution failed").get_int("a")
        }

        pub fn session_expire(user: DataObject) -> DataObject {
            let mut d = DataObject::new();
            d.put_object("user", user);
            ::flowlang::rustcmd::RustCmd::new("lvvzvn183bd066566j4").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn stream_write(uuid: String, stream_id: i64, data: DataBytes) -> bool {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_int("stream_id", stream_id);
            d.put_bytes("data", data);
            ::flowlang::rustcmd::RustCmd::new("pmumpq18d39a2594cp3").execute(d).expect("Rust command execution failed").get_boolean("a")
        }

        pub fn tcp_connect(uuid: String, ipaddr: String, port: i64) -> bool {
            let mut d = DataObject::new();
            d.put_string("uuid", &uuid);
            d.put_string("ipaddr", &ipaddr);
            d.put_int("port", port);
            ::flowlang::rustcmd::RustCmd::new("ltnpiq18385ba6cc7u3").execute(d).expect("Rust command execution failed").get_boolean("a")
        }

        pub fn udp_connect(ipaddr: String, port: i64) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("ipaddr", &ipaddr);
            d.put_int("port", port);
            ::flowlang::rustcmd::RustCmd::new("gloivk183adf03115od").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
    pub mod peer_select {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
}

pub mod runtime {
}

pub mod scratch {
    pub mod scratch {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

    }
}

pub mod security {
    pub mod security {
        use ::ndata::dataobject::DataObject;
        use ::ndata::dataarray::DataArray;
        use ::ndata::databytes::DataBytes;
        use ::ndata::data::Data;

        pub fn current_user(nn_sessionid: String) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("nn_sessionid", &nn_sessionid);
            ::flowlang::rustcmd::RustCmd::new("ihxsxh18410251dfapf7").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn deleteuser(id: String) -> String {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            ::flowlang::rustcmd::RustCmd::new("jszjgy1836bfe023ckc").execute(d).expect("Rust command execution failed").get_string("a")
        }

        pub fn groups() -> DataArray {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("qjmvtm1836b1bc850o9").execute(d).expect("Rust command execution failed").get_array("a")
        }

        pub fn init() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("suvlkp1846cfa2235q2c").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn setuser(id: String, displayname: String, password: String, groups: DataArray, keepalive: Data, address: Data, port: Data) -> DataObject {
            let mut d = DataObject::new();
            d.put_string("id", &id);
            d.put_string("displayname", &displayname);
            d.put_string("password", &password);
            d.put_array("groups", groups);
            d.set_property("keepalive", keepalive);
            d.set_property("address", address);
            d.set_property("port", port);
            ::flowlang::rustcmd::RustCmd::new("soqxoo1836bb51d5dy2").execute(d).expect("Rust command execution failed").get_object("a")
        }

        pub fn users() -> DataObject {
            let d = DataObject::new();
            ::flowlang::rustcmd::RustCmd::new("ysnihn1836b0814aen5").execute(d).expect("Rust command execution failed").get_object("a")
        }

    }
}

pub struct old_agent_agent {}
pub struct old_agent_llm {}
pub struct old_agent_plugin {}
pub struct old_agent_scratch {}
pub struct old_agent_agentloop {}
pub struct old_agent_agentprompt {}
pub struct old_agent_memory {}
pub struct old_agent_archivist {}
pub struct old_agent_chat {}
pub struct old_agent_askrow {}
pub struct old_agent_describebtn {}
pub struct old_agent_prompts {}
pub struct old_agent_executive {}
pub struct old_agent_sensor {}
pub struct old_agent_model {}
pub struct old_agent_msg {}
pub struct old_agent_context {}
pub struct old_agent_tools {}
pub struct old_app_api {}
pub struct old_app_app {}
pub struct old_app_appcard {}
pub struct old_app_appinfo {}
pub struct old_app_dial {}
pub struct old_app_list {}
pub struct old_app_list_item {}
pub struct old_app_login {}
pub struct old_app_scenegraph {}
pub struct old_app_select {}
pub struct old_app_service {}
pub struct old_app_shape {}
pub struct old_app_ui {}
pub struct old_app_ui_reference {}
pub struct old_app_util {}
pub struct old_app_sceneplayer {}
pub struct old_app_sceneexpr {}
pub struct old_app_scenetokens {}
pub struct old_app_scenedoc {}
pub struct old_app_sceneproject {}
pub struct old_app_scenerun {}
pub struct old_app_forcelayout {}
pub struct old_app_tokens {}
pub struct old_app_webgl {}
pub struct old_dev_dev {}
pub struct old_dev_editcommand {}
pub struct old_dev_editcontrol {}
pub struct old_dev_github {}
pub struct old_dev_libsettings {}
pub struct old_dev_plugins {}
pub struct old_dev_workbench {}
pub struct old_dev_sceneeditor {}
pub struct old_dev_floweditor {}
pub struct old_dev_floweditor3d {}
pub struct old_dev_editor {}
pub struct old_dev_preview {}
pub struct old_dev_shelf {}
pub struct old_dev_card {}
pub struct old_dev_jump {}
pub struct old_dev_frame {}
pub struct old_dev_toast {}
pub struct old_dev_session {}
pub struct old_dev_flowdoc {}
pub struct old_dev_flowproject {}
pub struct old_dev_flowprims {}
pub struct old_dev_flowlayout {}
pub struct old_dev_facets {}
pub struct old_dev_code {}
pub struct old_dev_viewctx {}
pub struct old_dev_git {}
pub struct old_kb_platform_api {}
pub struct old_kb_workflow {}
pub struct old_kb_frontend {}
pub struct old_kb_m2026_07 {}
pub struct old_kb_doctrine {}
pub struct old_kb_nebula {}
pub struct old_kb_camera {}
pub struct old_peer_headsup {}
pub struct old_peer_peer {}
pub struct old_peer_peer_model {}
pub struct old_peer_reboot {}
pub struct old_peer_service {}
pub struct old_peer_peer_select {}
pub struct old_scratch_scratch {}
pub struct old_security_security {}
pub struct old_agent {
    pub agent: old_agent_agent,
    pub llm: old_agent_llm,
    pub plugin: old_agent_plugin,
    pub scratch: old_agent_scratch,
    pub agentloop: old_agent_agentloop,
    pub agentprompt: old_agent_agentprompt,
    pub memory: old_agent_memory,
    pub archivist: old_agent_archivist,
    pub chat: old_agent_chat,
    pub askrow: old_agent_askrow,
    pub describebtn: old_agent_describebtn,
    pub prompts: old_agent_prompts,
    pub executive: old_agent_executive,
    pub sensor: old_agent_sensor,
    pub model: old_agent_model,
    pub msg: old_agent_msg,
    pub context: old_agent_context,
    pub tools: old_agent_tools,
}
pub struct old_app {
    pub api: old_app_api,
    pub app: old_app_app,
    pub appcard: old_app_appcard,
    pub appinfo: old_app_appinfo,
    pub dial: old_app_dial,
    pub list: old_app_list,
    pub list_item: old_app_list_item,
    pub login: old_app_login,
    pub scenegraph: old_app_scenegraph,
    pub select: old_app_select,
    pub service: old_app_service,
    pub shape: old_app_shape,
    pub ui: old_app_ui,
    pub ui_reference: old_app_ui_reference,
    pub util: old_app_util,
    pub sceneplayer: old_app_sceneplayer,
    pub sceneexpr: old_app_sceneexpr,
    pub scenetokens: old_app_scenetokens,
    pub scenedoc: old_app_scenedoc,
    pub sceneproject: old_app_sceneproject,
    pub scenerun: old_app_scenerun,
    pub forcelayout: old_app_forcelayout,
    pub tokens: old_app_tokens,
    pub webgl: old_app_webgl,
}
pub struct old_dev {
    pub dev: old_dev_dev,
    pub editcommand: old_dev_editcommand,
    pub editcontrol: old_dev_editcontrol,
    pub github: old_dev_github,
    pub libsettings: old_dev_libsettings,
    pub plugins: old_dev_plugins,
    pub workbench: old_dev_workbench,
    pub sceneeditor: old_dev_sceneeditor,
    pub floweditor: old_dev_floweditor,
    pub floweditor3d: old_dev_floweditor3d,
    pub editor: old_dev_editor,
    pub preview: old_dev_preview,
    pub shelf: old_dev_shelf,
    pub card: old_dev_card,
    pub jump: old_dev_jump,
    pub frame: old_dev_frame,
    pub toast: old_dev_toast,
    pub session: old_dev_session,
    pub flowdoc: old_dev_flowdoc,
    pub flowproject: old_dev_flowproject,
    pub flowprims: old_dev_flowprims,
    pub flowlayout: old_dev_flowlayout,
    pub facets: old_dev_facets,
    pub code: old_dev_code,
    pub viewctx: old_dev_viewctx,
    pub git: old_dev_git,
}
pub struct old_kb {
    pub platform_api: old_kb_platform_api,
    pub workflow: old_kb_workflow,
    pub frontend: old_kb_frontend,
    pub m2026_07: old_kb_m2026_07,
    pub doctrine: old_kb_doctrine,
    pub nebula: old_kb_nebula,
    pub camera: old_kb_camera,
}
pub struct old_peer {
    pub headsup: old_peer_headsup,
    pub peer: old_peer_peer,
    pub peer_model: old_peer_peer_model,
    pub reboot: old_peer_reboot,
    pub service: old_peer_service,
    pub peer_select: old_peer_peer_select,
}
pub struct old_runtime {
}
pub struct old_scratch {
    pub scratch: old_scratch_scratch,
}
pub struct old_security {
    pub security: old_security_security,
}
pub struct api {
    pub agent: old_agent,
    pub app: old_app,
    pub dev: old_dev,
    pub kb: old_kb,
    pub peer: old_peer,
    pub runtime: old_runtime,
    pub scratch: old_scratch,
    pub security: old_security,
}

pub const fn new() -> api {
    api {
        agent: old_agent {
            agent: old_agent_agent {},
            llm: old_agent_llm {},
            plugin: old_agent_plugin {},
            scratch: old_agent_scratch {},
            agentloop: old_agent_agentloop {},
            agentprompt: old_agent_agentprompt {},
            memory: old_agent_memory {},
            archivist: old_agent_archivist {},
            chat: old_agent_chat {},
            askrow: old_agent_askrow {},
            describebtn: old_agent_describebtn {},
            prompts: old_agent_prompts {},
            executive: old_agent_executive {},
            sensor: old_agent_sensor {},
            model: old_agent_model {},
            msg: old_agent_msg {},
            context: old_agent_context {},
            tools: old_agent_tools {},
        },
        app: old_app {
            api: old_app_api {},
            app: old_app_app {},
            appcard: old_app_appcard {},
            appinfo: old_app_appinfo {},
            dial: old_app_dial {},
            list: old_app_list {},
            list_item: old_app_list_item {},
            login: old_app_login {},
            scenegraph: old_app_scenegraph {},
            select: old_app_select {},
            service: old_app_service {},
            shape: old_app_shape {},
            ui: old_app_ui {},
            ui_reference: old_app_ui_reference {},
            util: old_app_util {},
            sceneplayer: old_app_sceneplayer {},
            sceneexpr: old_app_sceneexpr {},
            scenetokens: old_app_scenetokens {},
            scenedoc: old_app_scenedoc {},
            sceneproject: old_app_sceneproject {},
            scenerun: old_app_scenerun {},
            forcelayout: old_app_forcelayout {},
            tokens: old_app_tokens {},
            webgl: old_app_webgl {},
        },
        dev: old_dev {
            dev: old_dev_dev {},
            editcommand: old_dev_editcommand {},
            editcontrol: old_dev_editcontrol {},
            github: old_dev_github {},
            libsettings: old_dev_libsettings {},
            plugins: old_dev_plugins {},
            workbench: old_dev_workbench {},
            sceneeditor: old_dev_sceneeditor {},
            floweditor: old_dev_floweditor {},
            floweditor3d: old_dev_floweditor3d {},
            editor: old_dev_editor {},
            preview: old_dev_preview {},
            shelf: old_dev_shelf {},
            card: old_dev_card {},
            jump: old_dev_jump {},
            frame: old_dev_frame {},
            toast: old_dev_toast {},
            session: old_dev_session {},
            flowdoc: old_dev_flowdoc {},
            flowproject: old_dev_flowproject {},
            flowprims: old_dev_flowprims {},
            flowlayout: old_dev_flowlayout {},
            facets: old_dev_facets {},
            code: old_dev_code {},
            viewctx: old_dev_viewctx {},
            git: old_dev_git {},
        },
        kb: old_kb {
            platform_api: old_kb_platform_api {},
            workflow: old_kb_workflow {},
            frontend: old_kb_frontend {},
            m2026_07: old_kb_m2026_07 {},
            doctrine: old_kb_doctrine {},
            nebula: old_kb_nebula {},
            camera: old_kb_camera {},
        },
        peer: old_peer {
            headsup: old_peer_headsup {},
            peer: old_peer_peer {},
            peer_model: old_peer_peer_model {},
            reboot: old_peer_reboot {},
            service: old_peer_service {},
            peer_select: old_peer_peer_select {},
        },
        runtime: old_runtime {
        },
        scratch: old_scratch {
            scratch: old_scratch_scratch {},
        },
        security: old_security {
            security: old_security_security {},
        },
    }
}

impl old_agent_llm {
    #[deprecated(note = "use api::agent::llm::ask_llm instead")]
    pub fn ask_llm(&self, prompt: String, system_prompt: Data) -> String {
        self::agent::llm::ask_llm(prompt, system_prompt)
    }
    #[deprecated(note = "use api::agent::llm::tool_loop instead")]
    pub fn tool_loop(&self, prompt: String) -> DataObject {
        self::agent::llm::tool_loop(prompt)
    }
    #[deprecated(note = "use api::agent::llm::chat_llm instead")]
    pub fn chat_llm(&self, messages: DataArray, tools: DataArray) -> DataObject {
        self::agent::llm::chat_llm(messages, tools)
    }
    #[deprecated(note = "use api::agent::llm::claude_code instead")]
    pub fn claude_code(&self, messages: DataArray, tools: DataArray) -> DataObject {
        self::agent::llm::claude_code(messages, tools)
    }
}
impl old_agent_plugin {
    #[deprecated(note = "use api::agent::plugin::control_query instead")]
    pub fn control_query(&self, message: String, context: DataObject) -> String {
        self::agent::plugin::control_query(message, context)
    }
    #[deprecated(note = "use api::agent::plugin::list_tools instead")]
    pub fn list_tools(&self) -> DataObject {
        self::agent::plugin::list_tools()
    }
    #[deprecated(note = "use api::agent::plugin::describe_command instead")]
    pub fn describe_command(&self, command_name: String, lang: String, returntype: String, groups: String, params: DataArray, imports: String, code: String, current_description: String) -> String {
        self::agent::plugin::describe_command(command_name, lang, returntype, groups, params, imports, code, current_description)
    }
}
impl old_agent_scratch {
    #[deprecated(note = "use api::agent::scratch::eval_pshkms19ee68b2a1ct46 instead")]
    pub fn eval_pshkms19ee68b2a1ct46(&self) -> DataObject {
        self::agent::scratch::eval_pshkms19ee68b2a1ct46()
    }
}
impl old_agent_archivist {
    #[deprecated(note = "use api::agent::archivist::log_turn instead")]
    pub fn log_turn(&self, venue: String, ask: String, reply: String, tools: String, author: String) -> DataObject {
        self::agent::archivist::log_turn(venue, ask, reply, tools, author)
    }
    #[deprecated(note = "use api::agent::archivist::consolidate instead")]
    pub fn consolidate(&self) -> DataObject {
        self::agent::archivist::consolidate()
    }
    #[deprecated(note = "use api::agent::archivist::queue_status instead")]
    pub fn queue_status(&self) -> DataObject {
        self::agent::archivist::queue_status()
    }
    #[deprecated(note = "use api::agent::archivist::remember instead")]
    pub fn remember(&self, lib: String, domain: String, entry: DataObject, author: String) -> DataObject {
        self::agent::archivist::remember(lib, domain, entry, author)
    }
    #[deprecated(note = "use api::agent::archivist::promote instead")]
    pub fn promote(&self, lib: String) -> DataObject {
        self::agent::archivist::promote(lib)
    }
    #[deprecated(note = "use api::agent::archivist::seed_export instead")]
    pub fn seed_export(&self, domains: String, path: String) -> DataObject {
        self::agent::archivist::seed_export(domains, path)
    }
    #[deprecated(note = "use api::agent::archivist::bootstrap instead")]
    pub fn bootstrap(&self, path: String) -> DataObject {
        self::agent::archivist::bootstrap(path)
    }
    #[deprecated(note = "use api::agent::archivist::recall instead")]
    pub fn recall(&self, query: String, domains: String, limit: i64) -> DataObject {
        self::agent::archivist::recall(query, domains, limit)
    }
    #[deprecated(note = "use api::agent::archivist::adjudicate instead")]
    pub fn adjudicate(&self, lib: String, domain: String, entry: DataObject, author: String) -> DataObject {
        self::agent::archivist::adjudicate(lib, domain, entry, author)
    }
    #[deprecated(note = "use api::agent::archivist::epistemic_work instead")]
    pub fn epistemic_work(&self) -> DataObject {
        self::agent::archivist::epistemic_work()
    }
    #[deprecated(note = "use api::agent::archivist::decay instead")]
    pub fn decay(&self, lib: String, domain: String, claim: String, author: String) -> DataObject {
        self::agent::archivist::decay(lib, domain, claim, author)
    }
    #[deprecated(note = "use api::agent::archivist::reverify instead")]
    pub fn reverify(&self, limit: i64) -> DataObject {
        self::agent::archivist::reverify(limit)
    }
    #[deprecated(note = "use api::agent::archivist::connect instead")]
    pub fn connect(&self, subject: String) -> DataObject {
        self::agent::archivist::connect(subject)
    }
    #[deprecated(note = "use api::agent::archivist::wonder instead")]
    pub fn wonder(&self) -> DataObject {
        self::agent::archivist::wonder()
    }
}
impl old_agent_chat {
    #[deprecated(note = "use api::agent::chat::upload instead")]
    pub fn upload(&self, filename: String, data_b64: String) -> DataObject {
        self::agent::chat::upload(filename, data_b64)
    }
}
impl old_agent_executive {
    #[deprecated(note = "use api::agent::executive::start instead")]
    pub fn start(&self) -> DataObject {
        self::agent::executive::start()
    }
    #[deprecated(note = "use api::agent::executive::stop instead")]
    pub fn stop(&self) -> DataObject {
        self::agent::executive::stop()
    }
    #[deprecated(note = "use api::agent::executive::status instead")]
    pub fn status(&self) -> DataObject {
        self::agent::executive::status()
    }
    #[deprecated(note = "use api::agent::executive::perceive instead")]
    pub fn perceive(&self, perception: DataObject) -> DataObject {
        self::agent::executive::perceive(perception)
    }
    #[deprecated(note = "use api::agent::executive::set_drive instead")]
    pub fn set_drive(&self, acts_per_hour: i64) -> DataObject {
        self::agent::executive::set_drive(acts_per_hour)
    }
    #[deprecated(note = "use api::agent::executive::salience_log instead")]
    pub fn salience_log(&self) -> DataObject {
        self::agent::executive::salience_log()
    }
    #[deprecated(note = "use api::agent::executive::consolidate_room instead")]
    pub fn consolidate_room(&self, min_quiet_s: i64, window: i64, budget: i64) -> DataObject {
        self::agent::executive::consolidate_room(min_quiet_s, window, budget)
    }
}
impl old_agent_sensor {
    #[deprecated(note = "use api::agent::sensor::start instead")]
    pub fn start(&self) -> DataObject {
        self::agent::sensor::start()
    }
    #[deprecated(note = "use api::agent::sensor::stop instead")]
    pub fn stop(&self) -> DataObject {
        self::agent::sensor::stop()
    }
    #[deprecated(note = "use api::agent::sensor::status instead")]
    pub fn status(&self) -> DataObject {
        self::agent::sensor::status()
    }
    #[deprecated(note = "use api::agent::sensor::system_sense instead")]
    pub fn system_sense(&self) -> DataObject {
        self::agent::sensor::system_sense()
    }
    #[deprecated(note = "use api::agent::sensor::git_sense instead")]
    pub fn git_sense(&self) -> DataObject {
        self::agent::sensor::git_sense()
    }
}
impl old_agent_model {
    #[deprecated(note = "use api::agent::model::salience instead")]
    pub fn salience(&self, perception: DataObject, context: DataObject) -> DataObject {
        self::agent::model::salience(perception, context)
    }
    #[deprecated(note = "use api::agent::model::service_status instead")]
    pub fn service_status(&self) -> DataObject {
        self::agent::model::service_status()
    }
    #[deprecated(note = "use api::agent::model::curriculum_export instead")]
    pub fn curriculum_export(&self, path: String) -> DataObject {
        self::agent::model::curriculum_export(path)
    }
    #[deprecated(note = "use api::agent::model::bootstrap instead")]
    pub fn bootstrap(&self) -> DataObject {
        self::agent::model::bootstrap()
    }
    #[deprecated(note = "use api::agent::model::train_status instead")]
    pub fn train_status(&self) -> DataObject {
        self::agent::model::train_status()
    }
    #[deprecated(note = "use api::agent::model::get_settings instead")]
    pub fn get_settings(&self) -> DataObject {
        self::agent::model::get_settings()
    }
    #[deprecated(note = "use api::agent::model::set_setting instead")]
    pub fn set_setting(&self, key: String, value: String) -> DataObject {
        self::agent::model::set_setting(key, value)
    }
    #[deprecated(note = "use api::agent::model::promote_pointer instead")]
    pub fn promote_pointer(&self) -> DataObject {
        self::agent::model::promote_pointer()
    }
    #[deprecated(note = "use api::agent::model::metrics instead")]
    pub fn metrics(&self) -> DataObject {
        self::agent::model::metrics()
    }
    #[deprecated(note = "use api::agent::model::service_stop instead")]
    pub fn service_stop(&self) -> DataObject {
        self::agent::model::service_stop()
    }
    #[deprecated(note = "use api::agent::model::user_promote instead")]
    pub fn user_promote(&self) -> DataObject {
        self::agent::model::user_promote()
    }
    #[deprecated(note = "use api::agent::model::user_rollback instead")]
    pub fn user_rollback(&self) -> DataObject {
        self::agent::model::user_rollback()
    }
    #[deprecated(note = "use api::agent::model::persona_rederive instead")]
    pub fn persona_rederive(&self) -> DataObject {
        self::agent::model::persona_rederive()
    }
    #[deprecated(note = "use api::agent::model::persona_read instead")]
    pub fn persona_read(&self) -> DataObject {
        self::agent::model::persona_read()
    }
    #[deprecated(note = "use api::agent::model::persona_write instead")]
    pub fn persona_write(&self, content: String) -> DataObject {
        self::agent::model::persona_write(content)
    }
    #[deprecated(note = "use api::agent::model::import instead")]
    pub fn import(&self, name: String, source: String, backend: String, anchor: String) -> DataObject {
        self::agent::model::import(name, source, backend, anchor)
    }
    #[deprecated(note = "use api::agent::model::models instead")]
    pub fn models(&self) -> DataObject {
        self::agent::model::models()
    }
    #[deprecated(note = "use api::agent::model::model_remove instead")]
    pub fn model_remove(&self, name: String, purge: bool) -> DataObject {
        self::agent::model::model_remove(name, purge)
    }
    #[deprecated(note = "use api::agent::model::resources instead")]
    pub fn resources(&self) -> DataObject {
        self::agent::model::resources()
    }
    #[deprecated(note = "use api::agent::model::dataset_add instead")]
    pub fn dataset_add(&self, name: String, source: String, kind: String, format: String, holdout_every: i64, mode: String) -> DataObject {
        self::agent::model::dataset_add(name, source, kind, format, holdout_every, mode)
    }
    #[deprecated(note = "use api::agent::model::dataset_list instead")]
    pub fn dataset_list(&self) -> DataObject {
        self::agent::model::dataset_list()
    }
    #[deprecated(note = "use api::agent::model::dataset_inspect instead")]
    pub fn dataset_inspect(&self, name: String, peek: i64, verify: bool) -> DataObject {
        self::agent::model::dataset_inspect(name, peek, verify)
    }
    #[deprecated(note = "use api::agent::model::dataset_snapshot instead")]
    pub fn dataset_snapshot(&self, name: String, snapshot_name: String) -> DataObject {
        self::agent::model::dataset_snapshot(name, snapshot_name)
    }
    #[deprecated(note = "use api::agent::model::dataset_derive instead")]
    pub fn dataset_derive(&self, name: String, out_name: String, transform: String, limit: i64) -> DataObject {
        self::agent::model::dataset_derive(name, out_name, transform, limit)
    }
    #[deprecated(note = "use api::agent::model::dataset_remove instead")]
    pub fn dataset_remove(&self, name: String, purge: bool) -> DataObject {
        self::agent::model::dataset_remove(name, purge)
    }
    #[deprecated(note = "use api::agent::model::adapter_derive instead")]
    pub fn adapter_derive(&self, name: String, dataset: String, base: String, targets: String, rank: i64, steps: i64) -> DataObject {
        self::agent::model::adapter_derive(name, dataset, base, targets, rank, steps)
    }
    #[deprecated(note = "use api::agent::model::adapter_apply instead")]
    pub fn adapter_apply(&self, name: String, on: bool) -> DataObject {
        self::agent::model::adapter_apply(name, on)
    }
    #[deprecated(note = "use api::agent::model::adapters instead")]
    pub fn adapters(&self) -> DataObject {
        self::agent::model::adapters()
    }
    #[deprecated(note = "use api::agent::model::adapter_delete instead")]
    pub fn adapter_delete(&self, name: String) -> DataObject {
        self::agent::model::adapter_delete(name)
    }
    #[deprecated(note = "use api::agent::model::recipe_author instead")]
    pub fn recipe_author(&self, name: String, base: String, mix: String, posture: String, steps: i64, lr: String, evals: String, notes: String) -> DataObject {
        self::agent::model::recipe_author(name, base, mix, posture, steps, lr, evals, notes)
    }
    #[deprecated(note = "use api::agent::model::recipe_clone instead")]
    pub fn recipe_clone(&self, name: String, from: String, edits: DataObject) -> DataObject {
        self::agent::model::recipe_clone(name, from, edits)
    }
    #[deprecated(note = "use api::agent::model::recipes instead")]
    pub fn recipes(&self) -> DataObject {
        self::agent::model::recipes()
    }
    #[deprecated(note = "use api::agent::model::recipe_remove instead")]
    pub fn recipe_remove(&self, name: String) -> DataObject {
        self::agent::model::recipe_remove(name)
    }
    #[deprecated(note = "use api::agent::model::experiment instead")]
    pub fn experiment(&self, name: String, control: String, variant: String, budget_steps: i64) -> DataObject {
        self::agent::model::experiment(name, control, variant, budget_steps)
    }
    #[deprecated(note = "use api::agent::model::experiments instead")]
    pub fn experiments(&self) -> DataObject {
        self::agent::model::experiments()
    }
    #[deprecated(note = "use api::agent::model::sft_run instead")]
    pub fn sft_run(&self, name: String, dataset: String, base: String, rank: i64, steps: i64) -> DataObject {
        self::agent::model::sft_run(name, dataset, base, rank, steps)
    }
    #[deprecated(note = "use api::agent::model::sft_promote instead")]
    pub fn sft_promote(&self, checkpoint: String) -> DataObject {
        self::agent::model::sft_promote(checkpoint)
    }
    #[deprecated(note = "use api::agent::model::dataset_feed instead")]
    pub fn dataset_feed(&self, name: String, kind: String, lines: String, lineage: String, provenance: String, holdout_every: i64) -> DataObject {
        self::agent::model::dataset_feed(name, kind, lines, lineage, provenance, holdout_every)
    }
    #[deprecated(note = "use api::agent::model::why_harvest instead")]
    pub fn why_harvest(&self, source: String, repo_path: String, limit: i64) -> DataObject {
        self::agent::model::why_harvest(source, repo_path, limit)
    }
    #[deprecated(note = "use api::agent::model::harvest_report instead")]
    pub fn harvest_report(&self, window_days: i64) -> DataObject {
        self::agent::model::harvest_report(window_days)
    }
}
impl old_agent_msg {
    #[deprecated(note = "use api::agent::msg::put instead")]
    pub fn put(&self, role: String, venue: String, content: String, entity: String, provenance: String, id: String) -> DataObject {
        self::agent::msg::put(role, venue, content, entity, provenance, id)
    }
    #[deprecated(note = "use api::agent::msg::get instead")]
    pub fn get(&self, id: String) -> DataObject {
        self::agent::msg::get(id)
    }
    #[deprecated(note = "use api::agent::msg::recent instead")]
    pub fn recent(&self, venue: String, limit: i64) -> DataObject {
        self::agent::msg::recent(venue, limit)
    }
}
impl old_agent_context {
    #[deprecated(note = "use api::agent::context::assemble instead")]
    pub fn assemble(&self, purpose: String, subject: String, budget: i64) -> DataObject {
        self::agent::context::assemble(purpose, subject, budget)
    }
}
impl old_agent_tools {
    #[deprecated(note = "use api::agent::tools::ssh_run instead")]
    pub fn ssh_run(&self, host: String, cmd: String, timeout_secs: i64) -> DataObject {
        self::agent::tools::ssh_run(host, cmd, timeout_secs)
    }
    #[deprecated(note = "use api::agent::tools::rsync_push instead")]
    pub fn rsync_push(&self, host: String, src: String, dst: String) -> DataObject {
        self::agent::tools::rsync_push(host, src, dst)
    }
}
impl old_app_app {
    #[deprecated(note = "use api::app::app::apps instead")]
    pub fn apps(&self) -> DataArray {
        self::app::app::apps()
    }
    #[deprecated(note = "use api::app::app::asset instead")]
    pub fn asset(&self, nn_path: String) -> String {
        self::app::app::asset(nn_path)
    }
    #[deprecated(note = "use api::app::app::assets instead")]
    pub fn assets(&self, lib: String) -> DataArray {
        self::app::app::assets(lib)
    }
    #[deprecated(note = "use api::app::app::delete instead")]
    pub fn delete(&self, lib: String, id: String, nn_sessionid: String) -> String {
        self::app::app::delete(lib, id, nn_sessionid)
    }
    #[deprecated(note = "use api::app::app::deletelib instead")]
    pub fn deletelib(&self, lib: String) -> String {
        self::app::app::deletelib(lib)
    }
    #[deprecated(note = "use api::app::app::deviceid instead")]
    pub fn deviceid(&self) -> String {
        self::app::app::deviceid()
    }
    #[deprecated(note = "use api::app::app::eventoff instead")]
    pub fn eventoff(&self, id: String) -> String {
        self::app::app::eventoff(id)
    }
    #[deprecated(note = "use api::app::app::eventon instead")]
    pub fn eventon(&self, id: String, app: String, event: String, cmdlib: String, cmdid: String) -> String {
        self::app::app::eventon(id, app, event, cmdlib, cmdid)
    }
    #[deprecated(note = "use api::app::app::events instead")]
    pub fn events(&self, app: String) -> DataArray {
        self::app::app::events(app)
    }
    #[deprecated(note = "use api::app::app::exec instead")]
    pub fn exec(&self, lib: String, id: String, args: DataObject, nn_sessionid: String) -> DataObject {
        self::app::app::exec(lib, id, args, nn_sessionid)
    }
    #[deprecated(note = "use api::app::app::jsapi instead")]
    pub fn jsapi(&self, nn_path: String) -> DataObject {
        self::app::app::jsapi(nn_path)
    }
    #[deprecated(note = "use api::app::app::libs instead")]
    pub fn libs(&self) -> DataArray {
        self::app::app::libs()
    }
    #[deprecated(note = "use api::app::app::login instead")]
    pub fn login(&self, user: String, pass: String, nn_sessionid: String) -> DataObject {
        self::app::app::login(user, pass, nn_sessionid)
    }
    #[deprecated(note = "use api::app::app::newlib instead")]
    pub fn newlib(&self, lib: String, readers: DataArray, writers: DataArray) -> String {
        self::app::app::newlib(lib, readers, writers)
    }
    #[deprecated(note = "use api::app::app::read instead")]
    pub fn read(&self, lib: String, id: String, nn_sessionid: String) -> DataObject {
        self::app::app::read(lib, id, nn_sessionid)
    }
    #[deprecated(note = "use api::app::app::remembersession instead")]
    pub fn remembersession(&self, nn_session: DataObject) -> String {
        self::app::app::remembersession(nn_session)
    }
    #[deprecated(note = "use api::app::app::settings instead")]
    pub fn settings(&self, settings: Data) -> DataObject {
        self::app::app::settings(settings)
    }
    #[deprecated(note = "use api::app::app::spawn instead")]
    pub fn spawn(&self, lib: String, ctl: String, cmd: String, args: DataObject) -> DataObject {
        self::app::app::spawn(lib, ctl, cmd, args)
    }
    #[deprecated(note = "use api::app::app::timeroff instead")]
    pub fn timeroff(&self, id: String) -> String {
        self::app::app::timeroff(id)
    }
    #[deprecated(note = "use api::app::app::timeron instead")]
    pub fn timeron(&self, id: String, data: DataObject) -> String {
        self::app::app::timeron(id, data)
    }
    #[deprecated(note = "use api::app::app::uninstall instead")]
    pub fn uninstall(&self, app: String) -> String {
        self::app::app::uninstall(app)
    }
    #[deprecated(note = "use api::app::app::unique_session_id instead")]
    pub fn unique_session_id(&self) -> String {
        self::app::app::unique_session_id()
    }
    #[deprecated(note = "use api::app::app::write instead")]
    pub fn write(&self, lib: String, id: Data, data: DataObject, readers: Data, writers: Data, nn_sessionid: String) -> DataObject {
        self::app::app::write(lib, id, data, readers, writers, nn_sessionid)
    }
}
impl old_app_service {
    #[deprecated(note = "use api::app::service::init instead")]
    pub fn init(&self) -> String {
        self::app::service::init()
    }
}
impl old_app_util {
    #[deprecated(note = "use api::app::util::hash instead")]
    pub fn hash(&self, file: String) -> String {
        self::app::util::hash(file)
    }
    #[deprecated(note = "use api::app::util::init instead")]
    pub fn init(&self) -> String {
        self::app::util::init()
    }
    #[deprecated(note = "use api::app::util::zip instead")]
    pub fn zip(&self, srcdir: String, destfile: String) -> bool {
        self::app::util::zip(srcdir, destfile)
    }
}
impl old_dev_dev {
    #[deprecated(note = "use api::dev::dev::check instead")]
    pub fn check(&self, lib: String, ctl: String, cmd: String) -> String {
        self::dev::dev::check(lib, ctl, cmd)
    }
    #[deprecated(note = "use api::dev::dev::compile instead")]
    pub fn compile(&self, lib: String, ctl: String, cmd: String) -> DataObject {
        self::dev::dev::compile(lib, ctl, cmd)
    }
    #[deprecated(note = "use api::dev::dev::compile_rust instead")]
    pub fn compile_rust(&self) -> String {
        self::dev::dev::compile_rust()
    }
    #[deprecated(note = "use api::dev::dev::install_lib instead")]
    pub fn install_lib(&self, uuid: String, lib: String) -> bool {
        self::dev::dev::install_lib(uuid, lib)
    }
    #[deprecated(note = "use api::dev::dev::lib_archive instead")]
    pub fn lib_archive(&self, lib: String, version: i64) -> String {
        self::dev::dev::lib_archive(lib, version)
    }
    #[deprecated(note = "use api::dev::dev::lib_info instead")]
    pub fn lib_info(&self, lib: String) -> DataObject {
        self::dev::dev::lib_info(lib)
    }
    #[deprecated(note = "use api::dev::dev::rebuild_lib instead")]
    pub fn rebuild_lib(&self, lib: String) -> String {
        self::dev::dev::rebuild_lib(lib)
    }
    #[deprecated(note = "use api::dev::dev::activate_lib instead")]
    pub fn activate_lib(&self, lib: String) -> String {
        self::dev::dev::activate_lib(lib)
    }
}
impl old_dev_editcommand {
    #[deprecated(note = "use api::dev::editcommand::compile_command instead")]
    pub fn compile_command(&self, lib: String, control_name: String, cmd_name: String) -> DataObject {
        self::dev::editcommand::compile_command(lib, control_name, cmd_name)
    }
    #[deprecated(note = "use api::dev::editcommand::delete_command instead")]
    pub fn delete_command(&self, lib: String, control_id: String, cmd_id: String, nn_sessionid: String) -> String {
        self::dev::editcommand::delete_command(lib, control_id, cmd_id, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::editcommand::save_command instead")]
    pub fn save_command(&self, lib: String, cmd_id: String, lang: String, code: String, imports: String, returntype: String, params: DataArray, desc: String, groups: String, readers: DataArray, nn_sessionid: String) -> String {
        self::dev::editcommand::save_command(lib, cmd_id, lang, code, imports, returntype, params, desc, groups, readers, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::editcommand::read_command instead")]
    pub fn read_command(&self, lib: String, ctl: String, cmd: String) -> DataObject {
        self::dev::editcommand::read_command(lib, ctl, cmd)
    }
    #[deprecated(note = "use api::dev::editcommand::lookup_cmd_id instead")]
    pub fn lookup_cmd_id(&self, lib: String, ctl: String, cmd: String) -> String {
        self::dev::editcommand::lookup_cmd_id(lib, ctl, cmd)
    }
}
impl old_dev_editcontrol {
    #[deprecated(note = "use api::dev::editcontrol::add_component instead")]
    pub fn add_component(&self, lib: String, control_id: String, component_type: String, name: String, nn_sessionid: String) -> DataObject {
        self::dev::editcontrol::add_component(lib, control_id, component_type, name, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::editcontrol::appdata instead")]
    pub fn appdata(&self, data: DataObject) -> DataObject {
        self::dev::editcontrol::appdata(data)
    }
    #[deprecated(note = "use api::dev::editcontrol::get_control instead")]
    pub fn get_control(&self, lib: String, id: String) -> DataObject {
        self::dev::editcontrol::get_control(lib, id)
    }
    #[deprecated(note = "use api::dev::editcontrol::get_publish_context instead")]
    pub fn get_publish_context(&self, lib: String, control_id: String, nn_sessionid: String) -> DataObject {
        self::dev::editcontrol::get_publish_context(lib, control_id, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::editcontrol::lookup_id instead")]
    pub fn lookup_id(&self, lib: String, name: String) -> String {
        self::dev::editcontrol::lookup_id(lib, name)
    }
    #[deprecated(note = "use api::dev::editcontrol::publishapp instead")]
    pub fn publishapp(&self, data: DataObject) -> DataArray {
        self::dev::editcontrol::publishapp(data)
    }
    #[deprecated(note = "use api::dev::editcontrol::save_control instead")]
    pub fn save_control(&self, lib: String, id: String, html: String, css: String, js: String, groups: String, desc: String, readers: DataArray, inline_data: DataObject, nn_sessionid: String) -> String {
        self::dev::editcontrol::save_control(lib, id, html, css, js, groups, desc, readers, inline_data, nn_sessionid)
    }
}
impl old_dev_github {
    #[deprecated(note = "use api::dev::github::import instead")]
    pub fn import(&self, url: String) -> String {
        self::dev::github::import(url)
    }
    #[deprecated(note = "use api::dev::github::list instead")]
    pub fn list(&self) -> DataObject {
        self::dev::github::list()
    }
    #[deprecated(note = "use api::dev::github::update instead")]
    pub fn update(&self, lib: String) -> String {
        self::dev::github::update(lib)
    }
    #[deprecated(note = "use api::dev::github::remove instead")]
    pub fn remove(&self, lib: String, delete_repository: bool) -> String {
        self::dev::github::remove(lib, delete_repository)
    }
}
impl old_dev_libsettings {
    #[deprecated(note = "use api::dev::libsettings::get_library_config instead")]
    pub fn get_library_config(&self, id: String) -> DataObject {
        self::dev::libsettings::get_library_config(id)
    }
    #[deprecated(note = "use api::dev::libsettings::save_library_config instead")]
    pub fn save_library_config(&self, data: DataObject) -> DataObject {
        self::dev::libsettings::save_library_config(data)
    }
}
impl old_dev_plugins {
    #[deprecated(note = "use api::dev::plugins::list_plugins instead")]
    pub fn list_plugins(&self) -> DataObject {
        self::dev::plugins::list_plugins()
    }
}
impl old_dev_code {
    #[deprecated(note = "use api::dev::code::list_commands instead")]
    pub fn list_commands(&self, lib: String, ctl: String) -> DataArray {
        self::dev::code::list_commands(lib, ctl)
    }
    #[deprecated(note = "use api::dev::code::list_controls instead")]
    pub fn list_controls(&self, lib: String) -> DataArray {
        self::dev::code::list_controls(lib)
    }
    #[deprecated(note = "use api::dev::code::list_libraries instead")]
    pub fn list_libraries(&self) -> DataArray {
        self::dev::code::list_libraries()
    }
    #[deprecated(note = "use api::dev::code::add_library instead")]
    pub fn add_library(&self, lib: String) -> String {
        self::dev::code::add_library(lib)
    }
    #[deprecated(note = "use api::dev::code::add_control instead")]
    pub fn add_control(&self, lib: String, ctl: String) -> String {
        self::dev::code::add_control(lib, ctl)
    }
    #[deprecated(note = "use api::dev::code::upsert_command instead")]
    pub fn upsert_command(&self, lib: String, ctl: String, cmd: String, lang: String, return_type: String, params: DataArray, imports: String, code_body: String) -> DataObject {
        self::dev::code::upsert_command(lib, ctl, cmd, lang, return_type, params, imports, code_body)
    }
    #[deprecated(note = "use api::dev::code::patch_command_body instead")]
    pub fn patch_command_body(&self, lib: String, ctl: String, cmd: String, old_snippet: String, new_snippet: String) -> DataObject {
        self::dev::code::patch_command_body(lib, ctl, cmd, old_snippet, new_snippet)
    }
    #[deprecated(note = "use api::dev::code::read_command instead")]
    pub fn read_command(&self, lib: String, ctl: String, cmd: String) -> DataObject {
        self::dev::code::read_command(lib, ctl, cmd)
    }
    #[deprecated(note = "use api::dev::code::delete_command instead")]
    pub fn delete_command(&self, lib: String, ctl: String, cmd: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::delete_command(lib, ctl, cmd, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::search_commands instead")]
    pub fn search_commands(&self, lib: String, ctl: String, query: String) -> DataArray {
        self::dev::code::search_commands(lib, ctl, query)
    }
    #[deprecated(note = "use api::dev::code::invoke_command instead")]
    pub fn invoke_command(&self, lib: String, ctl: String, cmd: String, args: DataObject) -> DataObject {
        self::dev::code::invoke_command(lib, ctl, cmd, args)
    }
    #[deprecated(note = "use api::dev::code::evaluate_rust instead")]
    pub fn evaluate_rust(&self, imports: String, code: String) -> DataObject {
        self::dev::code::evaluate_rust(imports, code)
    }
    #[deprecated(note = "use api::dev::code::read_control_facet instead")]
    pub fn read_control_facet(&self, lib: String, ctl: String, facet: String) -> DataObject {
        self::dev::code::read_control_facet(lib, ctl, facet)
    }
    #[deprecated(note = "use api::dev::code::patch_control_facet instead")]
    pub fn patch_control_facet(&self, lib: String, ctl: String, facet: String, old_snippet: String, new_snippet: String, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::patch_control_facet(lib, ctl, facet, old_snippet, new_snippet, base, label, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::list_control_patches instead")]
    pub fn list_control_patches(&self, lib: String, ctl: String, limit: i64) -> DataObject {
        self::dev::code::list_control_patches(lib, ctl, limit)
    }
    #[deprecated(note = "use api::dev::code::set_library_meta instead")]
    pub fn set_library_meta(&self, lib: String, desc: String, groups: String) -> DataObject {
        self::dev::code::set_library_meta(lib, desc, groups)
    }
    #[deprecated(note = "use api::dev::code::set_control_meta instead")]
    pub fn set_control_meta(&self, lib: String, ctl: String, desc: String, groups: String) -> DataObject {
        self::dev::code::set_control_meta(lib, ctl, desc, groups)
    }
    #[deprecated(note = "use api::dev::code::set_command_meta instead")]
    pub fn set_command_meta(&self, lib: String, ctl: String, cmd: String, desc: String, groups: String) -> DataObject {
        self::dev::code::set_command_meta(lib, ctl, cmd, desc, groups)
    }
    #[deprecated(note = "use api::dev::code::list_assets instead")]
    pub fn list_assets(&self, lib: String) -> DataObject {
        self::dev::code::list_assets(lib)
    }
    #[deprecated(note = "use api::dev::code::write_asset instead")]
    pub fn write_asset(&self, lib: String, name: String, content: String, tempfile: String) -> DataObject {
        self::dev::code::write_asset(lib, name, content, tempfile)
    }
    #[deprecated(note = "use api::dev::code::rename_asset instead")]
    pub fn rename_asset(&self, lib: String, from: String, to: String) -> DataObject {
        self::dev::code::rename_asset(lib, from, to)
    }
    #[deprecated(note = "use api::dev::code::delete_asset instead")]
    pub fn delete_asset(&self, lib: String, name: String) -> DataObject {
        self::dev::code::delete_asset(lib, name)
    }
    #[deprecated(note = "use api::dev::code::read_flow_body instead")]
    pub fn read_flow_body(&self, lib: String, ctl: String, cmd: String) -> DataObject {
        self::dev::code::read_flow_body(lib, ctl, cmd)
    }
    #[deprecated(note = "use api::dev::code::write_flow_body instead")]
    pub fn write_flow_body(&self, lib: String, ctl: String, cmd: String, body: DataObject, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::write_flow_body(lib, ctl, cmd, body, base, label, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_timer instead")]
    pub fn set_timer(&self, lib: String, ctl: String, name: String, cmd: String, start: i64, startunit: String, interval: i64, intervalunit: String, repeat: bool, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_timer(lib, ctl, name, cmd, start, startunit, interval, intervalunit, repeat, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::remove_timer instead")]
    pub fn remove_timer(&self, lib: String, ctl: String, name: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::remove_timer(lib, ctl, name, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_event_handler instead")]
    pub fn set_event_handler(&self, lib: String, ctl: String, name: String, bot: String, event: String, cmd: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_event_handler(lib, ctl, name, bot, event, cmd, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::remove_event_handler instead")]
    pub fn remove_event_handler(&self, lib: String, ctl: String, name: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::remove_event_handler(lib, ctl, name, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::read_control_scene instead")]
    pub fn read_control_scene(&self, lib: String, ctl: String) -> DataObject {
        self::dev::code::read_control_scene(lib, ctl)
    }
    #[deprecated(note = "use api::dev::code::write_control_scene instead")]
    pub fn write_control_scene(&self, lib: String, ctl: String, scene: DataObject, base: String, label: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::write_control_scene(lib, ctl, scene, base, label, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::delete_library instead")]
    pub fn delete_library(&self, lib: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::delete_library(lib, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::delete_control instead")]
    pub fn delete_control(&self, lib: String, ctl: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::delete_control(lib, ctl, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::move_control instead")]
    pub fn move_control(&self, lib: String, ctl: String, to_lib: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::move_control(lib, ctl, to_lib, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_meta_identity instead")]
    pub fn set_meta_identity(&self, displayname: String, organization: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_meta_identity(displayname, organization, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::get_meta_identity instead")]
    pub fn get_meta_identity(&self) -> DataObject {
        self::dev::code::get_meta_identity()
    }
    #[deprecated(note = "use api::dev::code::unpublish_app instead")]
    pub fn unpublish_app(&self, lib: String, app: String, remove_runtime: bool, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::unpublish_app(lib, app, remove_runtime, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_plugin instead")]
    pub fn set_plugin(&self, name: String, target_lib: String, target_ctl: String, plugin_lib: String, plugin_ctl: String, selector: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_plugin(name, target_lib, target_ctl, plugin_lib, plugin_ctl, selector, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::remove_plugin instead")]
    pub fn remove_plugin(&self, name: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::remove_plugin(name, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_tags instead")]
    pub fn set_tags(&self, lib: String, ctl: String, cmd: String, tags: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_tags(lib, ctl, cmd, tags, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_groups instead")]
    pub fn set_groups(&self, lib: String, ctl: String, cmd: String, groups: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::code::set_groups(lib, ctl, cmd, groups, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::code::set_command_imports instead")]
    pub fn set_command_imports(&self, lib: String, ctl: String, cmd: String, imports: String) -> DataObject {
        self::dev::code::set_command_imports(lib, ctl, cmd, imports)
    }
    #[deprecated(note = "use api::dev::code::init instead")]
    pub fn init(&self) -> DataObject {
        self::dev::code::init()
    }
}
impl old_dev_git {
    #[deprecated(note = "use api::dev::git::gitrun instead")]
    pub fn gitrun(&self, repo: String, verb: String, args: DataArray, mode: String) -> DataObject {
        self::dev::git::gitrun(repo, verb, args, mode)
    }
    #[deprecated(note = "use api::dev::git::read instead")]
    pub fn read(&self, repo: String, verb: String, args: DataArray) -> DataObject {
        self::dev::git::read(repo, verb, args)
    }
    #[deprecated(note = "use api::dev::git::write instead")]
    pub fn write(&self, repo: String, verb: String, args: DataArray) -> DataObject {
        self::dev::git::write(repo, verb, args)
    }
    #[deprecated(note = "use api::dev::git::remote_op instead")]
    pub fn remote_op(&self, repo: String, verb: String, args: DataArray) -> DataObject {
        self::dev::git::remote_op(repo, verb, args)
    }
    #[deprecated(note = "use api::dev::git::set_repo instead")]
    pub fn set_repo(&self, name: String, path: String, origin: String, role: String, autocommit: bool, author: String, nn_sessionid: String) -> DataObject {
        self::dev::git::set_repo(name, path, origin, role, autocommit, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::git::remove_repo instead")]
    pub fn remove_repo(&self, name: String, author: String, nn_sessionid: String) -> DataObject {
        self::dev::git::remove_repo(name, author, nn_sessionid)
    }
    #[deprecated(note = "use api::dev::git::repos instead")]
    pub fn repos(&self) -> DataObject {
        self::dev::git::repos()
    }
    #[deprecated(note = "use api::dev::git::autocommit_sweep instead")]
    pub fn autocommit_sweep(&self) -> DataObject {
        self::dev::git::autocommit_sweep()
    }
}
impl old_peer_peer {
    #[deprecated(note = "use api::peer::peer::discovery instead")]
    pub fn discovery(&self) -> DataObject {
        self::peer::peer::discovery()
    }
    #[deprecated(note = "use api::peer::peer::info instead")]
    pub fn info(&self, nn_sessionid: String, uuid: Data, salt: Data) -> DataObject {
        self::peer::peer::info(nn_sessionid, uuid, salt)
    }
    #[deprecated(note = "use api::peer::peer::local instead")]
    pub fn local(&self, request: DataObject, nn_session: DataObject, nn_sessionid: String) -> DataObject {
        self::peer::peer::local(request, nn_session, nn_sessionid)
    }
    #[deprecated(note = "use api::peer::peer::peers instead")]
    pub fn peers(&self) -> DataArray {
        self::peer::peer::peers()
    }
    #[deprecated(note = "use api::peer::peer::remote instead")]
    pub fn remote(&self, nn_path: String, nn_params: DataObject, nn_headers: DataObject) -> DataBytes {
        self::peer::peer::remote(nn_path, nn_params, nn_headers)
    }
}
impl old_peer_reboot {
    #[deprecated(note = "use api::peer::reboot::init instead")]
    pub fn init(&self) -> DataObject {
        self::peer::reboot::init()
    }
    #[deprecated(note = "use api::peer::reboot::reboot instead")]
    pub fn reboot(&self) -> DataObject {
        self::peer::reboot::reboot()
    }
}
impl old_peer_service {
    #[deprecated(note = "use api::peer::service::close_stream instead")]
    pub fn close_stream(&self, uuid: String, streamid: i64, write: bool) -> DataObject {
        self::peer::service::close_stream(uuid, streamid, write)
    }
    #[deprecated(note = "use api::peer::service::discovery instead")]
    pub fn discovery(&self) -> String {
        self::peer::service::discovery()
    }
    #[deprecated(note = "use api::peer::service::exec instead")]
    pub fn exec(&self, uuid: String, app: String, cmd: String, params: DataObject) -> DataObject {
        self::peer::service::exec(uuid, app, cmd, params)
    }
    #[deprecated(note = "use api::peer::service::get_stream instead")]
    pub fn get_stream(&self, uuid: String, stream_id: i64) -> DataBytes {
        self::peer::service::get_stream(uuid, stream_id)
    }
    #[deprecated(note = "use api::peer::service::init instead")]
    pub fn init(&self) -> DataObject {
        self::peer::service::init()
    }
    #[deprecated(note = "use api::peer::service::listen instead")]
    pub fn listen(&self, ipaddr: String, port: i64) -> i64 {
        self::peer::service::listen(ipaddr, port)
    }
    #[deprecated(note = "use api::peer::service::listen_udp instead")]
    pub fn listen_udp(&self, ipaddr: String, port: i64) -> i64 {
        self::peer::service::listen_udp(ipaddr, port)
    }
    #[deprecated(note = "use api::peer::service::maintenance instead")]
    pub fn maintenance(&self) -> String {
        self::peer::service::maintenance()
    }
    #[deprecated(note = "use api::peer::service::new_stream instead")]
    pub fn new_stream(&self, uuid: String) -> i64 {
        self::peer::service::new_stream(uuid)
    }
    #[deprecated(note = "use api::peer::service::session_expire instead")]
    pub fn session_expire(&self, user: DataObject) -> DataObject {
        self::peer::service::session_expire(user)
    }
    #[deprecated(note = "use api::peer::service::stream_write instead")]
    pub fn stream_write(&self, uuid: String, stream_id: i64, data: DataBytes) -> bool {
        self::peer::service::stream_write(uuid, stream_id, data)
    }
    #[deprecated(note = "use api::peer::service::tcp_connect instead")]
    pub fn tcp_connect(&self, uuid: String, ipaddr: String, port: i64) -> bool {
        self::peer::service::tcp_connect(uuid, ipaddr, port)
    }
    #[deprecated(note = "use api::peer::service::udp_connect instead")]
    pub fn udp_connect(&self, ipaddr: String, port: i64) -> DataObject {
        self::peer::service::udp_connect(ipaddr, port)
    }
}
impl old_security_security {
    #[deprecated(note = "use api::security::security::current_user instead")]
    pub fn current_user(&self, nn_sessionid: String) -> DataObject {
        self::security::security::current_user(nn_sessionid)
    }
    #[deprecated(note = "use api::security::security::deleteuser instead")]
    pub fn deleteuser(&self, id: String) -> String {
        self::security::security::deleteuser(id)
    }
    #[deprecated(note = "use api::security::security::groups instead")]
    pub fn groups(&self) -> DataArray {
        self::security::security::groups()
    }
    #[deprecated(note = "use api::security::security::init instead")]
    pub fn init(&self) -> DataObject {
        self::security::security::init()
    }
    #[deprecated(note = "use api::security::security::setuser instead")]
    pub fn setuser(&self, id: String, displayname: String, password: String, groups: DataArray, keepalive: Data, address: Data, port: Data) -> DataObject {
        self::security::security::setuser(id, displayname, password, groups, keepalive, address, port)
    }
    #[deprecated(note = "use api::security::security::users instead")]
    pub fn users(&self) -> DataObject {
        self::security::security::users()
    }
}
