fn main() {
    tauri_build::build();
    generate_lan_dispatch();
}

fn generate_lan_dispatch() {
    use std::{collections::HashMap, fs, path::PathBuf};
    use syn::{FnArg, Item, Pat, ReturnType, Type};
    println!("cargo:rerun-if-changed=lan-commands.txt");
    let manifest = fs::read_to_string("lan-commands.txt").unwrap();
    let mut sources = HashMap::new();
    let mut dispatch = String::from("fn dispatch(database: &crate::db::Database, session: &crate::auth::AuthSession, command: &str, args: &serde_json::Value) -> RpcResult { match command {\n");
    let mut access =
        String::from("fn access(command: &str) -> Option<&'static str> { match command {\n");
    for line in manifest
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let (kind, path) = line.split_once(' ').expect("access and command path");
        assert!(["auth", "read", "write"].contains(&kind));
        let parts: Vec<_> = path.split("::").collect();
        let source = format!("src/{}/commands.rs", parts[0]);
        let ast = sources.entry(source.clone()).or_insert_with(|| {
            println!("cargo:rerun-if-changed={source}");
            syn::parse_file(&fs::read_to_string(&source).unwrap()).unwrap()
        });
        let function = ast
            .items
            .iter()
            .find_map(|item| match item {
                Item::Fn(f) if f.sig.ident == parts[2] => Some(f),
                _ => None,
            })
            .expect("allowlisted command must exist");
        let mut arguments = Vec::new();
        for arg in &function.sig.inputs {
            let FnArg::Typed(arg) = arg else {
                panic!("no self in commands")
            };
            let Pat::Ident(name) = &*arg.pat else {
                panic!("named arguments only")
            };
            let name = name.ident.to_string();
            if let Type::Path(ty) = &*arg.ty {
                if ty.path.segments.last().unwrap().ident == "State" {
                    assert!(["database", "session"].contains(&name.as_str()));
                    arguments.push(format!("crate::command_state::State({name})"));
                    continue;
                }
            }
            let mut camel = String::new();
            for (i, part) in name.split('_').enumerate() {
                if i == 0 {
                    camel.push_str(part);
                } else {
                    let mut chars = part.chars();
                    camel.extend(chars.next().unwrap().to_uppercase());
                    camel.extend(chars);
                }
            }
            arguments.push(format!("argument(args, {camel:?})?"));
        }
        let returns_result = match &function.sig.output {
            ReturnType::Type(_, ty) => {
                matches!(&**ty, Type::Path(p) if p.path.segments.last().unwrap().ident == "Result")
            }
            _ => false,
        };
        let encode = if returns_result {
            "encode_result"
        } else {
            "encode"
        };
        dispatch.push_str(&format!(
            "{:?} => {encode}(crate::{path}({})),\n",
            parts[2],
            arguments.join(", ")
        ));
        access.push_str(&format!("{:?} => Some({kind:?}),\n", parts[2]));
    }
    dispatch.push_str("_ => Err(error(\"unsupported_command\", \"This operation is not available through the server.\")), }}\n");
    access.push_str("_ => None, }}\n");
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("lan_dispatch.rs"),
        dispatch + &access,
    )
    .unwrap();
}
