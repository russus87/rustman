//! Esecuzione degli script pre/post in JavaScript con l'API `pm.*`, usando il
//! motore QuickJS (rquickjs). Replica la superficie di `src/lib/pm.js` del
//! frontend, così gli stessi script girano anche in CI.
//!
//! Esposti: `pm.variables`/`pm.environment` (get/set), `pm.request`,
//! `pm.response` (code/status/responseTime/text()/json()/headers.get),
//! `pm.test(nome, fn)`, `pm.expect(x).to.equal/eql/include/be.ok`, `console.log`.

use rquickjs::prelude::Func;
use rquickjs::{CatchResultExt, Context, Runtime};
use rustman_core::model::{Richiesta, Risposta, RisultatoTest};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Stato condiviso fra i callback nativi e lo script JS.
#[derive(Default)]
struct Stato {
    variabili: HashMap<String, String>,
    logs: Vec<String>,
    tests: Vec<RisultatoTest>,
}

/// Risultato dell'esecuzione di uno script.
pub struct EsitoScript {
    /// Variabili aggiornate (lo script può aver chiamato `pm.environment.set`).
    pub variabili: HashMap<String, String>,
    pub logs: Vec<String>,
    /// Asserzioni registrate con `pm.test(...)`.
    pub tests: Vec<RisultatoTest>,
}

/// Esegue uno script. `risposta` è presente solo per i post-script.
pub fn esegui(
    script: &str,
    variabili: &HashMap<String, String>,
    richiesta: &Richiesta,
    risposta: Option<&Risposta>,
) -> Result<EsitoScript, String> {
    let stato = Rc::new(RefCell::new(Stato {
        variabili: variabili.clone(),
        ..Default::default()
    }));

    let rt = Runtime::new().map_err(|e| e.to_string())?;
    let ctx = Context::full(&rt).map_err(|e| e.to_string())?;

    let req_json = serde_json::to_string(richiesta).unwrap_or_else(|_| "{}".into());
    let has_res = risposta.is_some();
    let res_json = risposta
        .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "null".into()))
        .unwrap_or_default();

    ctx.with(|ctx| -> Result<(), String> {
        let g = ctx.globals();

        let s = stato.clone();
        g.set(
            "__getVar",
            Func::from(move |k: String| s.borrow().variabili.get(&k).cloned().unwrap_or_default()),
        )
        .map_err(|e| e.to_string())?;

        let s = stato.clone();
        g.set(
            "__setVar",
            Func::from(move |k: String, v: String| {
                s.borrow_mut().variabili.insert(k, v);
            }),
        )
        .map_err(|e| e.to_string())?;

        let s = stato.clone();
        g.set(
            "__log",
            Func::from(move |m: String| {
                s.borrow_mut().logs.push(m);
            }),
        )
        .map_err(|e| e.to_string())?;

        let s = stato.clone();
        g.set(
            "__addTest",
            Func::from(move |nome: String, passato: bool, dettaglio: String| {
                s.borrow_mut().tests.push(RisultatoTest {
                    descrizione: nome,
                    passato,
                    dettaglio,
                });
            }),
        )
        .map_err(|e| e.to_string())?;

        g.set("__reqJson", req_json).map_err(|e| e.to_string())?;
        g.set("__resJson", res_json).map_err(|e| e.to_string())?;
        g.set("__hasRes", has_res).map_err(|e| e.to_string())?;

        ctx.eval::<(), _>(PRELUDE)
            .catch(&ctx)
            .map_err(|e| e.to_string())?;
        ctx.eval::<(), _>(script)
            .catch(&ctx)
            .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    let b = stato.borrow();
    Ok(EsitoScript {
        variabili: b.variabili.clone(),
        logs: b.logs.clone(),
        tests: b.tests.clone(),
    })
}

/// Bootstrap JS che definisce `console` e `pm` sopra i callback nativi.
const PRELUDE: &str = r#"
globalThis.console = {
  log:  function(){ __log(Array.prototype.map.call(arguments, String).join(' ')); },
  error:function(){ __log(Array.prototype.map.call(arguments, String).join(' ')); },
  warn: function(){ __log(Array.prototype.map.call(arguments, String).join(' ')); }
};
(function(){
  var req = {}; try { req = JSON.parse(__reqJson); } catch(e) {}
  var res = null; if (__hasRes) { try { res = JSON.parse(__resJson); } catch(e){} }
  function fmt(x){ return typeof x === 'string' ? x : JSON.stringify(x); }
  function expect(actual){
    function fail(m){ throw new Error(m); }
    return { to: {
      equal: function(e){ if(actual!==e) fail('atteso '+fmt(e)+', ottenuto '+fmt(actual)); return true; },
      eql: function(e){ if(JSON.stringify(actual)!==JSON.stringify(e)) fail('atteso '+fmt(e)+', ottenuto '+fmt(actual)); return true; },
      include: function(x){ var ok=(typeof actual==='string'||Array.isArray(actual))?actual.indexOf(x)>=0:(actual&&typeof actual==='object'&&(x in actual)); if(!ok) fail(fmt(actual)+' non include '+fmt(x)); return true; },
      get be(){ return {
        get ok(){ if(!actual) fail('valore non veritiero: '+fmt(actual)); return true; },
        get true(){ if(actual!==true) fail('atteso true, ottenuto '+fmt(actual)); return true; },
        get false(){ if(actual!==false) fail('atteso false, ottenuto '+fmt(actual)); return true; }
      }; }
    } };
  }
  var resObj = res ? {
    code: res.status, status: res.status_text, responseTime: res.tempo_ms,
    text: function(){ return res.body; },
    json: function(){ return JSON.parse(res.body); },
    headers: { get: function(n){ var f=(res.headers||[]).filter(function(h){return h.chiave.toLowerCase()===String(n).toLowerCase();})[0]; return f?f.valore:undefined; } }
  } : undefined;
  globalThis.pm = {
    variables:   { get: function(k){ return __getVar(k); }, set: function(k,v){ __setVar(k, String(v)); } },
    environment: { get: function(k){ return __getVar(k); }, set: function(k,v){ __setVar(k, String(v)); } },
    request: req,
    response: resObj,
    expect: expect,
    test: function(nome, fn){ try { fn(); __addTest(String(nome), true, ''); } catch(e){ __addTest(String(nome), false, (e&&e.message)?e.message:String(e)); } }
  };
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use rustman_core::model::Auth;

    fn richiesta_vuota() -> Richiesta {
        Richiesta {
            nome: "t".into(),
            metodo: "GET".into(),
            url: "https://x".into(),
            headers: vec![],
            params: vec![],
            auth: Auth::default(),
            body: String::new(),
            body_mode: "raw".into(),
            form: vec![],
            tests: vec![],
            pre_script: String::new(),
            post_script: String::new(),
        }
    }

    #[test]
    fn pre_script_imposta_variabile() {
        let vars = HashMap::new();
        let es = esegui(
            "pm.environment.set('ts', 42); console.log('ciao');",
            &vars,
            &richiesta_vuota(),
            None,
        )
        .unwrap();
        assert_eq!(es.variabili.get("ts"), Some(&"42".to_string()));
        assert!(es.logs.iter().any(|l| l.contains("ciao")));
    }

    #[test]
    fn post_script_test_su_risposta() {
        let risposta = Risposta {
            status: 200,
            status_text: "OK".into(),
            headers: vec![],
            body: "{\"id\":7}".into(),
            tempo_ms: 5,
            dimensione: 8,
        };
        let es = esegui(
            "pm.test('status ok', () => pm.expect(pm.response.code).to.equal(200));\n\
             pm.test('id', () => pm.expect(pm.response.json().id).to.equal(7));\n\
             pm.test('fallisce', () => pm.expect(1).to.equal(2));",
            &HashMap::new(),
            &richiesta_vuota(),
            Some(&risposta),
        )
        .unwrap();
        assert_eq!(es.tests.len(), 3);
        assert!(es.tests[0].passato);
        assert!(es.tests[1].passato);
        assert!(!es.tests[2].passato);
    }
}
