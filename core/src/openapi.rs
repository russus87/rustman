//! Import da specifiche OpenAPI 3.x (e Swagger 2.0 di base), in JSON o YAML.
//!
//! Modulo **puro** (nessun accesso al disco): riconosce lo spec e lo converte
//! in una collezione Rustman, raggruppando le operazioni per tag. La base URL
//! diventa la variabile `{{base_url}}` di un ambiente dedicato, e i parametri
//! di percorso `{id}` diventano segnaposto `{{id}}`.
//!
//! Cosa viene mappato:
//! - ogni `path` + metodo → una richiesta (nome = operationId/summary/METODO path);
//! - parametri `query`/`header`/`path`; corpo JSON d'esempio (dallo schema o dagli example);
//! - i `$ref` verso `components/schemas` (3.0) e `definitions` (2.0) vengono risolti.

use crate::model::{
    Asserzione, Auth, CampoForm, Environment, EsportaCollezione, Header, NodoExport, Richiesta,
    Variabile,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Riconosce e converte uno spec OpenAPI/Swagger. Restituisce la collezione e,
/// se è stata trovata una base URL, l'ambiente con `base_url`. `None` se il
/// contenuto non è uno spec valido.
pub fn riconosci(contenuto: &str) -> Option<(EsportaCollezione, Option<Environment>)> {
    // Prima JSON, poi YAML (gli spec OpenAPI sono spesso scritti in YAML).
    let spec: Spec = serde_json::from_str(contenuto)
        .ok()
        .or_else(|| serde_yaml::from_str(contenuto).ok())?;

    // Deve dichiararsi come OpenAPI 3.x o Swagger 2.0.
    if spec.openapi.is_empty() && spec.swagger.is_empty() {
        return None;
    }

    Some(converti(spec))
}

fn converti(spec: Spec) -> (EsportaCollezione, Option<Environment>) {
    let nome = if spec.info.title.trim().is_empty() {
        "Importata da OpenAPI".to_string()
    } else {
        spec.info.title.clone()
    };

    let base = base_url(&spec);
    let comps = componenti(&spec);

    // Raggruppa le richieste per tag (ordine alfabetico, stabile).
    let mut per_tag: BTreeMap<String, Vec<NodoExport>> = BTreeMap::new();
    let mut senza_tag: Vec<NodoExport> = Vec::new();

    for (path, item) in &spec.paths {
        for (metodo, op) in item.operazioni() {
            let Some(op) = op else { continue };
            let richiesta = costruisci_richiesta(metodo, path, item, op, &comps);
            let nodo = NodoExport::Richiesta { richiesta };
            match op.tags.first() {
                Some(tag) if !tag.trim().is_empty() => {
                    per_tag.entry(tag.clone()).or_default().push(nodo)
                }
                _ => senza_tag.push(nodo),
            }
        }
    }

    let mut figli: Vec<NodoExport> = per_tag
        .into_iter()
        .map(|(tag, figli)| NodoExport::Cartella { nome: tag, figli })
        .collect();
    figli.extend(senza_tag);

    let env = base.map(|valore| Environment {
        nome: format!("{nome} (server)"),
        variabili: vec![Variabile {
            chiave: "base_url".into(),
            valore,
            segreto: false,
        }],
    });

    let collezione = EsportaCollezione {
        rustman: 1,
        nome,
        figli,
    };
    (collezione, env)
}

/// Determina la base URL: `servers[0].url` (3.0) oppure schema+host+basePath (2.0).
fn base_url(spec: &Spec) -> Option<String> {
    if let Some(s) = spec.servers.first() {
        if !s.url.trim().is_empty() {
            return Some(s.url.clone());
        }
    }
    if !spec.host.is_empty() {
        let schema = spec.schemes.first().map(|s| s.as_str()).unwrap_or("https");
        return Some(format!("{schema}://{}{}", spec.host, spec.base_path));
    }
    None
}

/// Raccoglie gli schemi riusabili da `components.schemas` (3.0) e `definitions` (2.0).
fn componenti(spec: &Spec) -> HashMap<String, Schema> {
    let mut m = spec.definitions.clone();
    m.extend(spec.components.schemas.clone());
    m
}

fn costruisci_richiesta(
    metodo: &str,
    path: &str,
    item: &PathItem,
    op: &Operation,
    comps: &HashMap<String, Schema>,
) -> Richiesta {
    let nome = if !op.operation_id.trim().is_empty() {
        op.operation_id.clone()
    } else if !op.summary.trim().is_empty() {
        op.summary.clone()
    } else {
        format!("{} {}", metodo.to_uppercase(), path)
    };

    // I parametri di percorso `{id}` diventano segnaposto `{{id}}`.
    let url = format!("{{{{base_url}}}}{}", path.replace('{', "{{").replace('}', "}}"));

    // I parametri di percorso/cartella + operazione (l'operazione ha priorità).
    let mut headers: Vec<Header> = Vec::new();
    let mut params: Vec<Header> = Vec::new();
    let mut body = String::new();

    for p in item.parameters.iter().chain(op.parameters.iter()) {
        match p.posizione.as_str() {
            "query" => params.push(Header {
                chiave: p.name.clone(),
                valore: esempio_parametro(p),
                attivo: true,
            }),
            "header" => headers.push(Header {
                chiave: p.name.clone(),
                valore: esempio_parametro(p),
                attivo: true,
            }),
            // Swagger 2.0: il corpo è un parametro con `in: body` e uno schema.
            "body" => {
                if let Some(s) = &p.schema {
                    body = serde_json::to_string_pretty(&esempio_da_schema(s, comps, 0))
                        .unwrap_or_default();
                }
            }
            _ => {}
        }
    }

    // Corpo da requestBody (OpenAPI 3.0): preferiamo il JSON.
    if body.is_empty() {
        if let Some(rb) = &op.request_body {
            if let Some(mt) = rb.content.get("application/json").or_else(|| rb.content.values().next()) {
                if let Some(val) = mt.esempio(comps) {
                    body = serde_json::to_string_pretty(&val).unwrap_or_default();
                }
            }
        }
    }

    if !body.is_empty() {
        headers.push(Header {
            chiave: "Content-Type".into(),
            valore: "application/json".into(),
            attivo: true,
        });
    }

    // Contract testing: dallo schema della risposta 2xx (JSON) creiamo
    // un'asserzione `schema` che valida il body della risposta.
    let tests = asserzione_contratto(op, comps).into_iter().collect();

    Richiesta {
        nome,
        metodo: metodo.to_uppercase(),
        url,
        headers,
        params,
        auth: Auth::default(),
        body,
        body_mode: "raw".into(),
        form: Vec::<CampoForm>::new(),
        tests,
        pre_script: String::new(),
        post_script: String::new(),
    }
}

/// Crea (se possibile) un'asserzione di tipo `schema` dallo schema della prima
/// risposta 2xx con corpo JSON dell'operazione.
fn asserzione_contratto(op: &Operation, comps: &HashMap<String, Schema>) -> Option<Asserzione> {
    let resp = op
        .responses
        .iter()
        .find(|(code, _)| code.starts_with('2'))
        .map(|(_, r)| r)?;
    // OpenAPI 3: content["application/json"].schema; Swagger 2.0: response.schema.
    let schema = resp
        .content
        .get("application/json")
        .or_else(|| resp.content.values().next())
        .and_then(|mt| mt.schema.as_ref())
        .or(resp.schema.as_ref())?;

    let json_schema = schema_a_jsonschema(schema, comps, 0);
    Some(Asserzione {
        tipo: "schema".into(),
        operatore: String::new(),
        campo: String::new(),
        atteso: serde_json::to_string(&json_schema).unwrap_or_else(|_| "{}".into()),
        attivo: true,
    })
}

/// Converte uno `Schema` OpenAPI in un JSON Schema autonomo (con $ref risolti).
fn schema_a_jsonschema(s: &Schema, comps: &HashMap<String, Schema>, depth: u8) -> Value {
    if depth > 6 {
        return json!({});
    }
    if !s.riferimento.is_empty() {
        let nome = s.riferimento.rsplit('/').next().unwrap_or("");
        return match comps.get(nome) {
            Some(t) => schema_a_jsonschema(t, comps, depth + 1),
            None => json!({}),
        };
    }
    let mut out = serde_json::Map::new();
    if !s.tipo.is_empty() {
        out.insert("type".into(), json!(s.tipo));
    }
    if s.nullable {
        out.insert("nullable".into(), json!(true));
    }
    if !s.enumerazione.is_empty() {
        out.insert("enum".into(), json!(s.enumerazione));
    }
    if !s.required.is_empty() {
        out.insert("required".into(), json!(s.required));
    }
    if !s.properties.is_empty() {
        let props: serde_json::Map<String, Value> = s
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), schema_a_jsonschema(v, comps, depth + 1)))
            .collect();
        out.insert("properties".into(), Value::Object(props));
    }
    if let Some(items) = &s.items {
        out.insert("items".into(), schema_a_jsonschema(items, comps, depth + 1));
    }
    Value::Object(out)
}

/// Valore d'esempio per un parametro (query/header).
fn esempio_parametro(p: &Parameter) -> String {
    if let Some(e) = &p.example {
        return valore_scalare(e);
    }
    if let Some(s) = &p.schema {
        if let Some(e) = &s.example {
            return valore_scalare(e);
        }
    }
    String::new()
}

/// Converte uno scalare JSON in stringa "piatta" (senza virgolette).
fn valore_scalare(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        altro => altro.to_string(),
    }
}

/// Costruisce un valore d'esempio a partire da uno schema (con risoluzione $ref).
fn esempio_da_schema(s: &Schema, comps: &HashMap<String, Schema>, depth: u8) -> Value {
    if depth > 5 {
        return Value::Null;
    }
    // $ref → risolvi sul nome finale (es. "#/components/schemas/User" → "User").
    if !s.riferimento.is_empty() {
        let nome = s.riferimento.rsplit('/').next().unwrap_or("");
        return match comps.get(nome) {
            Some(target) => esempio_da_schema(target, comps, depth + 1),
            None => Value::Null,
        };
    }
    if let Some(e) = &s.example {
        return e.clone();
    }
    if let Some(primo) = s.enumerazione.first() {
        return primo.clone();
    }
    match s.tipo.as_str() {
        "object" => esempio_oggetto(s, comps, depth),
        "array" => {
            let elem = s
                .items
                .as_ref()
                .map(|it| esempio_da_schema(it, comps, depth + 1))
                .unwrap_or(Value::Null);
            Value::Array(vec![elem])
        }
        "integer" | "number" => Value::from(0),
        "boolean" => Value::Bool(true),
        "string" => Value::String(esempio_stringa(&s.format)),
        // Tipo assente ma con proprietà: trattalo come oggetto.
        _ if !s.properties.is_empty() => esempio_oggetto(s, comps, depth),
        _ => Value::Null,
    }
}

fn esempio_oggetto(s: &Schema, comps: &HashMap<String, Schema>, depth: u8) -> Value {
    let mut map = serde_json::Map::new();
    for (chiave, schema) in &s.properties {
        map.insert(chiave.clone(), esempio_da_schema(schema, comps, depth + 1));
    }
    Value::Object(map)
}

fn esempio_stringa(format: &str) -> String {
    match format {
        "date-time" => "2024-01-01T00:00:00Z".into(),
        "date" => "2024-01-01".into(),
        "email" => "nome@example.com".into(),
        "uuid" => "00000000-0000-0000-0000-000000000000".into(),
        _ => "string".into(),
    }
}

// ===================== Strutture OpenAPI (lette in modo tollerante) ==========

#[derive(Deserialize)]
struct Spec {
    #[serde(default)]
    openapi: String,
    #[serde(default)]
    swagger: String,
    #[serde(default)]
    info: Info,
    #[serde(default)]
    servers: Vec<Server>,
    #[serde(default)]
    host: String,
    #[serde(default, rename = "basePath")]
    base_path: String,
    #[serde(default)]
    schemes: Vec<String>,
    #[serde(default)]
    paths: BTreeMap<String, PathItem>,
    #[serde(default)]
    components: Components,
    #[serde(default)]
    definitions: HashMap<String, Schema>,
}

#[derive(Deserialize, Default)]
struct Info {
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct Server {
    #[serde(default)]
    url: String,
}

#[derive(Deserialize, Default)]
struct Components {
    #[serde(default)]
    schemas: HashMap<String, Schema>,
}

#[derive(Deserialize, Default)]
struct PathItem {
    #[serde(default)]
    get: Option<Operation>,
    #[serde(default)]
    post: Option<Operation>,
    #[serde(default)]
    put: Option<Operation>,
    #[serde(default)]
    patch: Option<Operation>,
    #[serde(default)]
    delete: Option<Operation>,
    #[serde(default)]
    head: Option<Operation>,
    #[serde(default)]
    options: Option<Operation>,
    #[serde(default)]
    parameters: Vec<Parameter>,
}

impl PathItem {
    /// Le operazioni del path, in ordine di metodo HTTP.
    fn operazioni(&self) -> Vec<(&'static str, Option<&Operation>)> {
        vec![
            ("get", self.get.as_ref()),
            ("post", self.post.as_ref()),
            ("put", self.put.as_ref()),
            ("patch", self.patch.as_ref()),
            ("delete", self.delete.as_ref()),
            ("head", self.head.as_ref()),
            ("options", self.options.as_ref()),
        ]
    }
}

#[derive(Deserialize)]
struct Operation {
    #[serde(default, rename = "operationId")]
    operation_id: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    parameters: Vec<Parameter>,
    #[serde(default, rename = "requestBody")]
    request_body: Option<RequestBody>,
    #[serde(default)]
    responses: BTreeMap<String, ResponseObj>,
}

#[derive(Deserialize)]
struct ResponseObj {
    #[serde(default)]
    content: HashMap<String, MediaType>,
    /// Swagger 2.0: lo schema è direttamente sulla response.
    #[serde(default)]
    schema: Option<Schema>,
}

#[derive(Deserialize)]
struct Parameter {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "in")]
    posizione: String,
    #[serde(default)]
    schema: Option<Schema>,
    #[serde(default)]
    example: Option<Value>,
}

#[derive(Deserialize)]
struct RequestBody {
    #[serde(default)]
    content: HashMap<String, MediaType>,
}

#[derive(Deserialize)]
struct MediaType {
    #[serde(default)]
    example: Option<Value>,
    #[serde(default)]
    examples: HashMap<String, EsempioOggetto>,
    #[serde(default)]
    schema: Option<Schema>,
}

impl MediaType {
    fn esempio(&self, comps: &HashMap<String, Schema>) -> Option<Value> {
        if let Some(e) = &self.example {
            return Some(e.clone());
        }
        if let Some(e) = self.examples.values().next().and_then(|x| x.value.clone()) {
            return Some(e);
        }
        self.schema.as_ref().map(|s| esempio_da_schema(s, comps, 0))
    }
}

#[derive(Deserialize)]
struct EsempioOggetto {
    #[serde(default)]
    value: Option<Value>,
}

#[derive(Deserialize, Default, Clone)]
struct Schema {
    #[serde(default, rename = "type")]
    tipo: String,
    #[serde(default)]
    properties: HashMap<String, Schema>,
    #[serde(default)]
    items: Option<Box<Schema>>,
    #[serde(default)]
    example: Option<Value>,
    #[serde(default, rename = "enum")]
    enumerazione: Vec<Value>,
    #[serde(default)]
    format: String,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    nullable: bool,
    #[serde(default, rename = "$ref")]
    riferimento: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"
openapi: 3.0.0
info:
  title: Pet API
servers:
  - url: https://api.pets.test/v1
paths:
  /pets/{petId}:
    get:
      operationId: getPet
      tags: [pets]
      parameters:
        - name: petId
          in: path
        - name: verbose
          in: query
          example: true
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Pet'
    post:
      operationId: addPet
      tags: [pets]
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/Pet'
components:
  schemas:
    Pet:
      type: object
      properties:
        id: { type: integer }
        nome: { type: string }
"#;

    #[test]
    fn importa_openapi_yaml() {
        let (coll, env) = riconosci(SPEC).expect("spec valida");
        assert_eq!(coll.nome, "Pet API");

        // base_url come ambiente.
        let env = env.expect("atteso ambiente con base_url");
        assert_eq!(env.variabili[0].chiave, "base_url");
        assert_eq!(env.variabili[0].valore, "https://api.pets.test/v1");

        // Cartella "pets" con due richieste.
        let NodoExport::Cartella { nome, figli } = &coll.figli[0] else {
            panic!("attesa una cartella per tag");
        };
        assert_eq!(nome, "pets");
        assert_eq!(figli.len(), 2);

        // GET con path param → {{petId}} e query param.
        let NodoExport::Richiesta { richiesta } = &figli[0] else {
            panic!();
        };
        assert_eq!(richiesta.nome, "getPet");
        assert_eq!(richiesta.url, "{{base_url}}/pets/{{petId}}");
        assert!(richiesta.params.iter().any(|p| p.chiave == "verbose"));
        // Contract testing: un'asserzione "schema" derivata dalla response 200.
        assert_eq!(richiesta.tests.len(), 1);
        assert_eq!(richiesta.tests[0].tipo, "schema");
        assert!(richiesta.tests[0].atteso.contains("properties"));

        // POST con body generato dallo schema risolto via $ref.
        let NodoExport::Richiesta { richiesta } = &figli[1] else {
            panic!();
        };
        assert_eq!(richiesta.metodo, "POST");
        assert!(richiesta.body.contains("\"nome\""));
        assert!(richiesta.body.contains("\"id\""));
    }

    #[test]
    fn non_openapi_ignorato() {
        assert!(riconosci(r#"{"info":{"name":"x"},"item":[]}"#).is_none());
        assert!(riconosci("ciao mondo").is_none());
    }
}
