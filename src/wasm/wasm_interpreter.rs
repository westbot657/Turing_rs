use std::fs;
use anyhow::{anyhow, Result};
use wasmi::*;
use wasmi::core::UntypedVal;
use wat;
use crate::data::beatmap_types::Beatmap;
use crate::data::game_objects::*;
use crate::{add_color_note_to_beatmap, create_color_note, get_beatmap};

struct HostState {

}

impl HostState {
    fn new() -> HostState {
        HostState {}
    }
}

pub struct WasmInterpreter {
    engine: Engine,
    store: Store<HostState>,
    linker: Linker<HostState>,
    script_instance: Option<(Module, Instance)>,
}

impl WasmInterpreter {
    pub fn new() -> WasmInterpreter {
        let mut config = Config::default();
        config.enforced_limits(EnforcedLimits::strict());
        let engine = Engine::new(&config);
        let mut store = Store::new(&engine, HostState::new());
        let mut linker = <Linker<HostState>>::new(&engine);

        unsafe {
            bind_data(&engine, &mut store, &mut linker).expect("Failed to setup wasm environment");
        }
        WasmInterpreter {
            engine,
            store,
            linker,
            script_instance: None,
        }
    }

    pub fn load_script(&mut self, path: &str) -> Result<()> {

        let data = fs::read_to_string(path)?;

        let wasm = wat::parse_str(&data)?;

        let module = Module::new(&self.engine, &mut &wasm[..])?;

        let instance = self.linker
            .instantiate(&mut self.store, &module)?
            .start(&mut self.store)?;

        self.script_instance = Some((module, instance));

        Ok(())
    }

    pub fn call_void_method(&mut self, name: &str) -> Result<()> {
        if let Some((_, instance)) = &self.script_instance {
            let init_function = instance.get_typed_func::<(), ()>(&self.store, name)?;
            init_function.call(&mut self.store, ())?;
            Ok(())
        } else {
            Err(anyhow!("no script is currently loaded"))
        }
    }

    pub fn call_init(&mut self) -> Result<()> {
        self.call_void_method("init")
    }

    pub fn call_end(&mut self) -> Result<()> {
        self.call_void_method("end")
    }

    pub fn call_update(&mut self) -> Result<()> {
        self.call_void_method("update")
    }

}


macro_rules! unpack_ref {
    ($store:ident, $var_in:ident => $var:ident : $typ:ty $body:block ) => {
        if let Some(macro_ref) = $var_in {
            if let Some(macro_any) = macro_ref.data($store) {
                let $var = macro_any.downcast_ref::<$typ>().unwrap();
                $body
            }
        }
    };
}

unsafe fn bind_data(engine: &Engine, store: &mut Store<HostState>, linker: &mut Linker<HostState>) -> Result<()> {

    // wasm names are prefixed with '_' so that languages
    // can have abstraction layers to turn stuff into normal
    // structures for the language, and use non-prefixed names

    // GLOBAL VARIABLES
    let beatmap: Beatmap = get_beatmap();
    let global_beatmap = Global::new(store, Val::ExternRef(ExternRef::new(store, beatmap)), Mutability::Const);
    linker.define("env", "_beatmap", global_beatmap)?;


    // GLOBAL FUNCTIONS
    let function_create_color_note = Func::wrap(store, |caller: Caller<'_, HostState>| {
        let note = create_color_note();
        Val::ExternRef(ExternRef::new(store, note))
    });
    linker.define("env", "_create_color_note", function_create_color_note)?;


    // INSTANCE FUNCTIONS
    let method_beatmap_add_note = Func::wrap(store, |caller: Caller<'_, HostState>, beatmap_opt: Option<ExternRef>, note_opt: Option<ExternRef>| {
        unpack_ref!(store, beatmap_opt => beatmap: Beatmap {
            unpack_ref!(store, note_opt => note: ColorNote {
                add_color_note_to_beatmap(note);
            })
        });

    });
    linker.define("env", "_beatmap_add_color_note", method_beatmap_add_note)?;


    Ok(())
}

#[cfg(test)]
mod wasm_tests {
    use std::fs;
    use anyhow::Result;
    use wasmprinter::print_bytes;

    #[test]
    fn test_wasm() -> Result<()> {
        let data = fs::read(r"C:\Users\Westb\Desktop\turing_wasm\target\wasm32-unknown-unknown\debug\turing_wasm.wasm")?;

        let wasm = wat::parse_bytes(&data)?;


        println!("{:#}", print_bytes(&wasm)?);

        Ok(())
    }
}
