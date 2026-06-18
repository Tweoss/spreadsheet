use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{BTreeMap, HashMap},
    ops::RangeInclusive,
    rc::Rc,
};

use rhai::{AST, Engine, EvalAltResult, NativeCallContext, Position, Scope};

// TODO: just make a RRef Scripts?
type RRef<T> = Rc<RefCell<T>>;

#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct Scripts(RRef<ScriptsInner>);

#[derive(serde::Deserialize, serde::Serialize)]
struct ScriptsInner {
    #[serde(skip)]
    engine: RRef<Engine>,
    scripts: BTreeMap<String, Script>,
    #[serde(skip)]
    values: HashMap<String, BTreeMap<usize, f64>>,
    #[serde(skip)]
    call_stack: Vec<(String, usize)>,
    range: RangeInclusive<usize>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Script {
    pub text: String,
    #[serde(skip)]
    pub ast: Option<AST>,
}

impl Default for ScriptsInner {
    fn default() -> Self {
        Self {
            engine: RefCell::new(Engine::new()).into(),
            scripts: (BTreeMap::new()),
            values: (HashMap::new()),
            call_stack: (Vec::new()),
            range: (0..=100),
        }
    }
}

impl Scripts {
    /// Necessary to wire up the new engine to the possibly deserialized old
    /// (but newly allocated) scripts.
    ///
    /// This happens because serde will use parts of the Default::default
    /// structure depending on what can be deserialized.
    pub fn init(&mut self) -> Result<(), String> {
        let clone = self.clone();
        self.0.borrow_mut().engine.borrow_mut().register_fn(
            "get",
            move |ctx: NativeCallContext, key: &str, row: f64| {
                let row = row.floor().max(0.0) as usize;
                clone.eval_one(Some(ctx.call_position()), key, row)
            },
        );
        self.eval()
    }

    pub fn eval(&mut self) -> Result<(), String> {
        let mut inner = self.0.borrow_mut();
        let old_values = inner.values.clone();
        for v in inner.values.values_mut() {
            v.clear();
        }
        let keys: Vec<String> = inner.scripts.keys().cloned().collect();
        let range = inner.range.clone();
        drop(inner);
        for i in range {
            for key in &keys {
                // eprintln!("top level call of {key} {i}");

                // In theory should be clear anyways.
                self.0.borrow_mut().call_stack.clear();

                match self.eval_one(None, key, i) {
                    Ok(_) => {}
                    Err(err) => {
                        // Reset values.
                        self.0.borrow_mut().values = old_values;
                        return Err(format!("In {key}[{i}], {err}"));
                    }
                };
            }
        }
        Ok(())
    }

    fn eval_one(
        &self,
        pos: Option<Position>,
        key: &str,
        row: usize,
    ) -> Result<f64, Box<EvalAltResult>> {
        let key = key.to_owned();
        // eprintln!(
        //     "eval one of {key} {row} at depth {}",
        //     self.call_stack.borrow().len()
        // );
        let mut inner = self.0.borrow_mut();
        let ScriptsInner {
            scripts,
            values,
            call_stack,
            range,
            engine,
        } = &mut *inner;
        if call_stack.contains(&(key.clone(), row)) {
            call_stack.push((key.clone(), row));
            let stack: Vec<String> = inner
                .call_stack
                .iter()
                .map(|s| format!("({}, {})", s.0, s.1))
                .collect();
            let stack = stack.join(" -> ");

            return Err(EvalAltResult::ErrorRuntime(
                ("Encountered dependency loop ".to_owned() + stack.as_str()).into(),
                pos.unwrap_or_default(),
            )
            .into());
        }
        call_stack.push((key.clone(), row));
        if !range.contains(&row) {
            return Err(EvalAltResult::ErrorRuntime(
                (format!(
                    "Row index {row} of \"{key}\" out of range [{},{}]",
                    range.start(),
                    range.end()
                ))
                .into(),
                pos.unwrap_or_default(),
            )
            .into());
        }
        if let Some(v) = values.get(&key).and_then(|v| v.get(&row)) {
            // eprintln!("eval one of {key} {row} read from cache");
            call_stack.pop();
            return Ok(*v);
        }
        let ast = if let Some(script) = scripts.get_mut(&key) {
            if let Some(ast) = &script.ast {
                ast.clone()
            } else {
                let ast = engine.borrow_mut().compile(wrap_script(&script.text))?;
                script.ast = Some(ast);
                script.ast.as_ref().unwrap().clone()
            }
        } else {
            return Err(
                EvalAltResult::ErrorModuleNotFound(key.to_owned(), Position::default()).into(),
            );
        };
        let engine = engine.clone();
        drop(inner);
        let value =
            engine
                .borrow()
                .call_fn::<f64>(&mut Scope::new(), &ast, "run", (row as f64,))?;
        let mut inner = self.0.borrow_mut();
        inner
            .values
            .entry(key.to_owned())
            .or_default()
            .insert(row, value);

        // eprintln!("eval one of {key} {row} finished eval");
        inner.call_stack.pop();

        Ok(value)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.borrow().scripts.contains_key(key)
    }
    pub fn remove_script(&mut self, key: &str) {
        self.0.borrow_mut().scripts.remove(key);
    }
    pub fn add_script(&mut self, key: String) {
        self.0.borrow_mut().scripts.insert(
            key,
            Script {
                text: String::new(),
                ast: None,
            },
        );
    }

    pub fn set_num_rows(&mut self, count: usize) -> Result<(), String> {
        if count > 0 {
            self.0.borrow_mut().range = 0..=(count - 1);
            self.eval()
        } else {
            Ok(())
        }
    }
    pub fn num_rows(&self) -> usize {
        self.0.borrow().range.end() + 1
    }

    pub fn key_count(&self) -> usize {
        self.0.borrow().scripts.keys().len()
    }

    pub fn nth_key(&self, n: usize) -> Option<String> {
        self.0.borrow().scripts.keys().nth(n).cloned()
    }

    pub fn borrow(&self) -> ScriptGuard<'_> {
        let guard = self.0.borrow();
        ScriptGuard { guard }
    }

    pub fn borrow_mut(&mut self) -> ScriptGuardMut<'_> {
        let guard = self.0.borrow_mut();
        ScriptGuardMut { guard }
    }
}

pub struct ScriptGuard<'a> {
    guard: Ref<'a, ScriptsInner>,
}
impl ScriptGuard<'_> {
    pub fn values(&self) -> &HashMap<String, BTreeMap<usize, f64>> {
        &self.guard.values
    }
}
pub struct ScriptGuardMut<'a> {
    guard: RefMut<'a, ScriptsInner>,
}
impl ScriptGuardMut<'_> {
    pub fn scripts(&mut self) -> &mut BTreeMap<String, Script> {
        &mut self.guard.scripts
    }
}

pub fn wrap_script(script: &str) -> String {
    "fn run(row) {\n".to_string() + script + "\n}"
}
