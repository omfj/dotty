use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::config::{Action, DottyConfig, Link};
use crate::utils::{self, is_on_path};

macro_rules! lua_try {
    ($e:expr) => {
        $e.map_err(|e| anyhow::anyhow!("{}", e))?
    };
}

pub fn load(path: &std::path::Path, config: DottyConfig) -> anyhow::Result<DottyConfig> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path.display(), e))?;

    let links: Arc<Mutex<Vec<Link>>> = Arc::new(Mutex::new(vec![]));
    let actions: Arc<Mutex<Vec<Action>>> = Arc::new(Mutex::new(vec![]));

    let lua = Lua::new();

    lua_try!(lua.globals().set("io", LuaNil));
    lua_try!(lua.globals().set("os", LuaNil));

    let dotty = lua_try!(lua.create_table());

    let hostname = utils::get_hostname();
    let os_name = utils::get_os_name();
    let profile = config.selected_profile.clone();

    lua_try!(dotty.set(
        "hostname",
        lua_try!(lua.create_function(move |_, ()| Ok(hostname.clone())))
    ));
    lua_try!(dotty.set(
        "os",
        lua_try!(lua.create_function(move |_, ()| Ok(os_name.clone())))
    ));
    lua_try!(dotty.set(
        "profile",
        lua_try!(lua.create_function(move |_, ()| Ok(profile.clone())))
    ));

    lua_try!(dotty.set(
        "test",
        lua_try!(lua.create_function(|_, cmd: String| Ok(is_on_path(&cmd))))
    ));

    lua_try!(dotty.set(
        "file_exists",
        lua_try!(lua.create_function(|_, path: String| {
            let expanded = if let Some(rest) = path.strip_prefix("~/") {
                dirs::home_dir()
                    .map(|h| h.join(rest).to_string_lossy().to_string())
                    .unwrap_or(path)
            } else {
                path
            };
            Ok(std::path::Path::new(&expanded).exists())
        }))
    ));

    let links_ref = links.clone();
    lua_try!(dotty.set(
        "link",
        lua_try!(
            lua.create_function(move |_, (source, target): (String, String)| {
                links_ref.lock().unwrap().push(Link { source, target });
                Ok(())
            })
        )
    ));

    let actions_ref = actions.clone();
    lua_try!(dotty.set(
        "run",
        lua_try!(
            lua.create_function(move |_, (name, arg): (String, LuaValue)| {
                let (command, shell) = match arg {
                    LuaValue::String(s) => (s.to_str()?.to_string(), String::new()),
                    LuaValue::Table(t) => {
                        let command: String = t.get("command")?;
                        let shell: String = t.get::<Option<String>>("shell")?.unwrap_or_default();
                        (command, shell)
                    }
                    _ => {
                        return Err(LuaError::runtime(
                            "run() expects a string or table as second argument",
                        ));
                    }
                };
                actions_ref.lock().unwrap().push(Action {
                    name,
                    command,
                    shell,
                });
                Ok(())
            })
        )
    ));

    lua_try!(lua.globals().set("dotty", dotty));

    lua_try!(
        lua.load(&src)
            .set_name(path.to_string_lossy().as_ref())
            .exec()
            .map_err(|e| anyhow::anyhow!("Lua error in '{}': {}", path.display(), e))
    );

    drop(lua);

    let collected_links = Arc::try_unwrap(links).unwrap().into_inner().unwrap();
    let collected_actions = Arc::try_unwrap(actions).unwrap().into_inner().unwrap();

    Ok(DottyConfig {
        links: collected_links,
        actions: collected_actions,
        ..config
    })
}
