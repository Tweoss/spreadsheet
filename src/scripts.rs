use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    ops::RangeInclusive,
    rc::Rc,
};

use rhai::{AST, Engine, EvalAltResult, NativeCallContext, Position, Scope};

// TODO: just make a RRef Scripts?
type RRef<T> = Rc<RefCell<T>>;
type REngine = RRef<Engine>;
type RScripts = RRef<BTreeMap<String, Script>>;
type RValues = RRef<HashMap<String, BTreeMap<usize, f64>>>;
type RCallStack = RRef<Vec<(String, usize)>>;
type RRange = RRef<RangeInclusive<usize>>;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Scripts {
    #[serde(skip)]
    engine: REngine,
    scripts: RScripts,
    values: RValues,
    #[serde(skip)]
    call_stack: RCallStack,
    range: RRange,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Script {
    pub text: String,
    #[serde(skip)]
    pub ast: Option<AST>,
}

impl Default for Scripts {
    fn default() -> Self {
        Self {
            engine: RefCell::new(Engine::new()).into(),
            scripts: RefCell::new(BTreeMap::new()).into(),
            values: RefCell::new(HashMap::new()).into(),
            call_stack: RefCell::new(Vec::new()).into(),
            range: RefCell::new(0..=100).into(),
        }
    }
}

impl Scripts {
    /// Necessary to wire up the new engine to the possibly deserialized old
    /// (but newly allocated) scripts.
    ///
    /// This happens because serde will use parts of the Default::default
    /// structure depending on what can be deserialized.
    pub fn init(&mut self) {
        let clone = self.clone();
        self.engine.borrow_mut().register_fn(
            "get",
            move |ctx: NativeCallContext, key: &str, row: f64| {
                let row = row.floor().max(0.0) as usize;
                clone.eval_one(Some(ctx.call_position()), key, row)
            },
        );
    }

    pub fn eval(&mut self) -> Result<(), Box<EvalAltResult>> {
        let old_values = self.values.borrow_mut().clone();
        for v in self.values.borrow_mut().values_mut() {
            v.clear();
        }
        let keys: Vec<String> = self.scripts.borrow().keys().cloned().collect();
        for i in self.range.borrow().clone() {
            for key in &keys {
                // eprintln!("top level call of {key} {i}");
                self.call_stack.borrow_mut().clear();
                match self.eval_one(None, key, i) {
                    Ok(_) => {}
                    Err(err) => {
                        // Reset values.
                        *self.values.borrow_mut() = old_values;
                        return Err(err);
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
        if self.call_stack.borrow().contains(&(key.clone(), row)) {
            self.call_stack.borrow_mut().push((key.clone(), row));
            let stack: Vec<String> = self
                .call_stack
                .borrow()
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
        self.call_stack.borrow_mut().push((key.clone(), row));
        if !self.range.borrow().contains(&row) {
            return Err(EvalAltResult::ErrorRuntime(
                (format!(
                    "Row index {row} of \"{key}\" out of range [{},{}]",
                    self.range.borrow().start(),
                    self.range.borrow().end()
                ))
                .into(),
                pos.unwrap_or_default(),
            )
            .into());
        }
        if let Some(v) = self.values.borrow().get(&key).and_then(|v| v.get(&row)) {
            // eprintln!("eval one of {key} {row} read from cache");
            self.call_stack.borrow_mut().pop();
            return Ok(*v);
        }
        let mut borrow = self.scripts.borrow_mut();
        let ast = if let Some(script) = borrow.get_mut(&key) {
            if let Some(ast) = &script.ast {
                ast.clone()
            } else {
                let ast = self.engine.borrow().compile(wrap_script(&script.text))?;
                script.ast = Some(ast);
                script.ast.as_ref().unwrap().clone()
            }
        } else {
            return Err(
                EvalAltResult::ErrorModuleNotFound(key.to_owned(), Position::default()).into(),
            );
        };
        drop(borrow);
        let value =
            self.engine
                .borrow()
                .call_fn::<f64>(&mut Scope::new(), &ast, "run", (row as f64,))?;
        self.values
            .borrow_mut()
            .entry(key.to_owned())
            .or_default()
            .insert(row, value);

        // eprintln!("eval one of {key} {row} finished eval");
        self.call_stack.borrow_mut().pop();

        Ok(value)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.scripts.borrow().contains_key(key)
    }
    pub fn remove_script(&mut self, key: &str) {
        self.scripts.borrow_mut().remove(key);
    }
    pub fn add_script(&mut self, key: String) {
        self.scripts.borrow_mut().insert(
            key,
            Script {
                text: String::new(),
                ast: None,
            },
        );
    }

    pub fn set_num_rows(&mut self, count: usize) -> Result<(), Box<EvalAltResult>> {
        if count > 0 {
            *self.range.borrow_mut() = 0..=(count - 1);
            self.eval()
        } else {
            Ok(())
        }
    }
    pub fn num_rows(&self) -> usize {
        self.range.borrow().end() + 1
    }

    pub fn key_count(&self) -> usize {
        self.scripts.borrow().keys().len()
    }

    pub fn nth_key(&self, n: usize) -> Option<String> {
        self.scripts.borrow().keys().nth(n).cloned()
    }

    pub fn scripts_mut(&self) -> std::cell::RefMut<'_, BTreeMap<String, Script>> {
        self.scripts.borrow_mut()
    }

    pub fn borrow_values(&self) -> std::cell::Ref<'_, HashMap<String, BTreeMap<usize, f64>>> {
        self.values.borrow()
    }
}

pub fn wrap_script(script: &str) -> String {
    "fn run(row) {\n".to_string() + script + "\n}"
}
