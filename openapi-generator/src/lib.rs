use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use schemars::{generate::SchemaSettings, JsonSchema, SchemaGenerator};
use serde_json::{Map, Value};

pub struct SchemaRegistry {
    generator: SchemaGenerator,
}

impl SchemaRegistry {
    #[must_use]
    pub fn openapi3() -> Self {
        Self {
            generator: SchemaSettings::openapi3().into_generator(),
        }
    }

    pub fn schema_ref<T: JsonSchema>(&mut self) -> Value {
        self.generator.subschema_for::<T>().to_value()
    }

    #[must_use]
    pub fn into_components(mut self) -> Value {
        Value::Object(self.generator.take_definitions(true))
    }
}

pub fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    deep_merge(base_value, overlay_value);
                } else {
                    base_map.insert(key, overlay_value);
                }
            }
        }
        (base_value, overlay_value) => *base_value = overlay_value,
    }
}

pub fn load_yaml(path: impl AsRef<Path>) -> Result<Value> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read YAML from {}", path.display()))?;
    let value = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse YAML from {}", path.display()))?;
    Ok(value)
}

pub fn to_yaml_string(value: &Value) -> Result<String> {
    let mut normalized = value.clone();
    sort_value(&mut normalized);
    let mut yaml = serde_yaml::to_string(&normalized).context("failed to serialize YAML")?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

pub fn write_or_check_yaml(path: impl AsRef<Path>, value: &Value, check: bool) -> Result<()> {
    let path = path.as_ref();
    let rendered = to_yaml_string(value)?;
    if check {
        let existing = fs::read_to_string(path)
            .with_context(|| format!("failed to read generated file {}", path.display()))?;
        if existing != rendered {
            bail!("generated OpenAPI file is stale: {}", path.display());
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, rendered)
        .with_context(|| format!("failed to write generated file {}", path.display()))?;
    Ok(())
}

#[must_use]
pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert(key.into(), value);
    }
    Value::Object(map)
}

pub struct ApiServer<'a> {
    pub url: &'a str,
    pub description: &'a str,
}

pub struct ApiInfo<'a> {
    pub title: &'a str,
    pub version: &'a str,
    pub description: &'a str,
    pub servers: &'a [ApiServer<'a>],
}

pub struct OperationRef<'a> {
    pub path: &'a str,
    pub leaf_path: &'a str,
}

pub enum ParameterLocation {
    Path,
    Query,
    Header,
}

pub enum HttpMethod {
    Get,
    Post,
}

pub struct NamedExample<'a> {
    pub name: &'a str,
    pub summary: Option<&'a str>,
    pub value: Value,
}

pub struct ParameterSpec<'a> {
    pub name: &'a str,
    pub location: ParameterLocation,
    pub required: bool,
    pub description: &'a str,
    pub schema: Value,
    pub example: Option<Value>,
}

pub enum RequestBodySpec<'a> {
    Json {
        schema: Value,
        required: bool,
        example: Option<Value>,
        examples: Vec<NamedExample<'a>>,
    },
}

pub enum ResponseContent<'a> {
    Json {
        schema: Value,
        example: Option<Value>,
        examples: Vec<NamedExample<'a>>,
    },
    Text {
        schema: Value,
        example: Option<&'a str>,
    },
}

pub struct ResponseSpec<'a> {
    pub status: &'a str,
    pub description: &'a str,
    pub content: Option<ResponseContent<'a>>,
}

pub struct OperationSpec<'a> {
    pub title: &'a str,
    pub path: &'a str,
    pub method: HttpMethod,
    pub operation_id: &'a str,
    pub summary: &'a str,
    pub description: &'a str,
    pub tags: &'a [&'a str],
    pub request_body: Option<RequestBodySpec<'a>>,
    pub responses: Vec<ResponseSpec<'a>>,
    pub components: Value,
}

pub struct AggregateOperationSpec<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub path: &'a str,
    pub method: HttpMethod,
    pub operation_id: &'a str,
    pub summary: &'a str,
    pub description: &'a str,
    pub tags: &'a [&'a str],
    pub parameters: Vec<ParameterSpec<'a>>,
    pub request_body: Option<RequestBodySpec<'a>>,
    pub responses: Vec<ResponseSpec<'a>>,
}

pub fn build_leaf_doc(info: &ApiInfo<'_>, operation: OperationSpec<'_>) -> Value {
    let method = operation_item_value(
        operation.operation_id,
        operation.summary,
        operation.description,
        operation.tags,
        vec![],
        operation.request_body,
        operation.responses,
        None,
        None,
    );

    Value::Object(Map::from_iter([
        ("openapi".into(), Value::String("3.0.3".to_string())),
        ("info".into(), leaf_info_value(info, operation.title)),
        (
            "paths".into(),
            object([(
                operation.path,
                object([(operation.method.as_str(), method)]),
            )]),
        ),
        (
            "components".into(),
            object([("schemas", operation.components)]),
        ),
        ("servers".into(), servers_value(info.servers)),
    ]))
}

pub fn build_aggregate_doc(info: &ApiInfo<'_>, operations: &[OperationRef<'_>]) -> Value {
    Value::Object(Map::from_iter([
        ("openapi".into(), Value::String("3.0.3".to_string())),
        ("info".into(), info_value(info)),
        (
            "paths".into(),
            Value::Object(
                operations
                    .iter()
                    .map(|operation| {
                        (
                            operation.path.to_string(),
                            object([(
                                "$ref",
                                Value::String(operation.leaf_path.to_string()),
                            )]),
                        )
                    })
                    .collect(),
            ),
        ),
        ("servers".into(), servers_value(info.servers)),
    ]))
}

pub fn build_service_doc(
    info: &ApiInfo<'_>,
    operations: Vec<AggregateOperationSpec<'_>>,
    components: Value,
) -> Value {
    let mut paths = Map::new();

    for operation in operations {
        let path_item = paths
            .entry(operation.path.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let path_methods = path_item.as_object_mut().unwrap();
        path_methods.insert(
            operation.method.as_str().to_string(),
            operation_item_value(
                operation.operation_id,
                operation.summary,
                operation.description,
                operation.tags,
                operation.parameters,
                operation.request_body,
                operation.responses,
                Some(operation.slug),
                Some(operation.title),
            ),
        );
    }

    Value::Object(Map::from_iter([
        ("openapi".into(), Value::String("3.0.3".to_string())),
        ("info".into(), info_value(info)),
        ("paths".into(), Value::Object(paths)),
        (
            "components".into(),
            object([("schemas", components)]),
        ),
        ("servers".into(), servers_value(info.servers)),
    ]))
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
        }
    }
}

impl ParameterLocation {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Header => "header",
        }
    }
}

fn info_value(info: &ApiInfo<'_>) -> Value {
    object([
        ("title", Value::String(info.title.to_string())),
        ("version", Value::String(info.version.to_string())),
        ("description", Value::String(info.description.to_string())),
    ])
}

fn leaf_info_value(info: &ApiInfo<'_>, title: &str) -> Value {
    object([
        ("title", Value::String(title.to_string())),
        ("version", Value::String(info.version.to_string())),
    ])
}

fn servers_value(servers: &[ApiServer<'_>]) -> Value {
    Value::Array(
        servers
            .iter()
            .map(|server| {
                object([
                    ("url", Value::String(server.url.to_string())),
                    (
                        "description",
                        Value::String(server.description.to_string()),
                    ),
                ])
            })
            .collect(),
    )
}

fn request_body_value(request_body: RequestBodySpec<'_>) -> Value {
    match request_body {
        RequestBodySpec::Json {
            schema,
            required,
            example,
            examples,
        } => object([
            ("required", Value::Bool(required)),
            (
                "content",
                object([(
                    "application/json",
                    media_type_value(schema, example, examples),
                )]),
            ),
        ]),
    }
}

fn operation_item_value(
    operation_id: &str,
    summary: &str,
    description: &str,
    tags: &[&str],
    parameters: Vec<ParameterSpec<'_>>,
    request_body: Option<RequestBodySpec<'_>>,
    responses: Vec<ResponseSpec<'_>>,
    slug: Option<&str>,
    title: Option<&str>,
) -> Value {
    let mut method = Map::new();
    method.insert("operationId".into(), Value::String(operation_id.to_string()));
    method.insert("summary".into(), Value::String(summary.to_string()));
    method.insert("description".into(), Value::String(description.to_string()));
    method.insert(
        "tags".into(),
        Value::Array(
            tags.iter()
                .map(|tag| Value::String((*tag).to_string()))
                .collect(),
        ),
    );

    if !parameters.is_empty() {
        method.insert(
            "parameters".into(),
            Value::Array(parameters.into_iter().map(parameter_value).collect()),
        );
    }

    if let Some(request_body) = request_body {
        method.insert("requestBody".into(), request_body_value(request_body));
    }

    let rendered_responses = responses
        .into_iter()
        .map(|response| (response.status.to_string(), response_value(response)))
        .collect::<Map<_, _>>();
    method.insert("responses".into(), Value::Object(rendered_responses));

    if let Some(slug) = slug {
        method.insert("x-fastnear-slug".into(), Value::String(slug.to_string()));
    }

    if let Some(title) = title {
        method.insert("x-fastnear-title".into(), Value::String(title.to_string()));
    }

    Value::Object(method)
}

fn parameter_value(parameter: ParameterSpec<'_>) -> Value {
    let mut rendered = Map::new();
    rendered.insert("name".into(), Value::String(parameter.name.to_string()));
    rendered.insert(
        "in".into(),
        Value::String(parameter.location.as_str().to_string()),
    );
    rendered.insert("required".into(), Value::Bool(parameter.required));
    rendered.insert(
        "description".into(),
        Value::String(parameter.description.to_string()),
    );
    rendered.insert("schema".into(), parameter.schema);

    if let Some(example) = parameter.example {
        rendered.insert("example".into(), example);
    }

    Value::Object(rendered)
}

fn response_value(response: ResponseSpec<'_>) -> Value {
    let mut value = Map::new();
    value.insert(
        "description".into(),
        Value::String(response.description.to_string()),
    );

    if let Some(content) = response.content {
        let content_value = match content {
            ResponseContent::Json {
                schema,
                example,
                examples,
            } => object([(
                "application/json",
                media_type_value(schema, example, examples),
            )]),
            ResponseContent::Text { schema, example } => {
                let mut text_media_type = object([("schema", schema)]);
                if let Some(example) = example {
                    text_media_type
                        .as_object_mut()
                        .unwrap()
                        .insert("example".into(), Value::String(example.to_string()));
                }
                object([("text/plain", text_media_type)])
            }
        };
        value.insert("content".into(), content_value);
    }

    Value::Object(value)
}

fn media_type_value(
    schema: Value,
    example: Option<Value>,
    examples: Vec<NamedExample<'_>>,
) -> Value {
    let mut media_type = Map::from_iter([("schema".into(), schema)]);

    if let Some(example) = example {
        media_type.insert("example".into(), example);
    }

    if !examples.is_empty() {
        media_type.insert(
            "examples".into(),
            Value::Object(
                examples
                    .into_iter()
                    .map(|example| {
                        let mut rendered = Map::new();
                        if let Some(summary) = example.summary {
                            rendered.insert(
                                "summary".into(),
                                Value::String(summary.to_string()),
                            );
                        }
                        rendered.insert("value".into(), example.value);
                        (example.name.to_string(), Value::Object(rendered))
                    })
                    .collect(),
            ),
        );
    }

    Value::Object(media_type)
}

fn sort_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
            for (_, child) in &mut entries {
                sort_value(child);
            }
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));

            let mut sorted = Map::new();
            for (key, child) in entries {
                sorted.insert(key, child);
            }
            *map = sorted;
        }
        Value::Array(items) => {
            for item in items {
                sort_value(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct NestedType {
        id: String,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct RootType {
        nested: NestedType,
    }

    #[test]
    fn deep_merge_merges_nested_objects_and_replaces_scalars() {
        let mut base = json!({
            "paths": {
                "/v0/transfers": {
                    "post": {
                        "summary": "before",
                        "responses": {
                            "200": {
                                "description": "before"
                            }
                        }
                    }
                }
            }
        });
        let overlay = json!({
            "paths": {
                "/v0/transfers": {
                    "post": {
                        "summary": "after",
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {}
                                }
                            }
                        }
                    }
                }
            }
        });

        deep_merge(&mut base, overlay);

        assert_eq!(base["paths"]["/v0/transfers"]["post"]["summary"], "after");
        assert_eq!(
            base["paths"]["/v0/transfers"]["post"]["responses"]["200"]["description"],
            "before"
        );
        assert!(base["paths"]["/v0/transfers"]["post"]["responses"]["200"]["content"].is_object());
    }

    #[test]
    fn schema_registry_collects_component_schemas() {
        let mut registry = SchemaRegistry::openapi3();
        let schema_ref = registry.schema_ref::<RootType>();
        let components = registry.into_components();

        assert_eq!(schema_ref["$ref"], "#/components/schemas/RootType");
        assert!(components["RootType"].is_object());
        assert!(components["NestedType"].is_object());
    }

    #[test]
    fn yaml_serialization_is_stable_for_the_same_value() {
        let value = json!({
            "openapi": "3.1.0",
            "paths": {
                "/v0/transfers": {
                    "post": {
                        "operationId": "get_transfers_by_account"
                    }
                }
            }
        });

        let first = to_yaml_string(&value).unwrap();
        let second = to_yaml_string(&value).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn aggregate_doc_includes_info_servers_and_refs() {
        let info = ApiInfo {
            title: "Example API",
            version: "1.0.0",
            description: "Example description",
            servers: &[ApiServer {
                url: "https://example.com",
                description: "Mainnet",
            }],
        };

        let doc = build_aggregate_doc(
            &info,
            &[OperationRef {
                path: "/v0/example",
                leaf_path: "./v0/example.yaml#/paths/~1v0~1example",
            }],
        );

        assert_eq!(doc["info"]["title"], "Example API");
        assert_eq!(doc["servers"][0]["url"], "https://example.com");
        assert_eq!(
            doc["paths"]["/v0/example"]["$ref"],
            "./v0/example.yaml#/paths/~1v0~1example"
        );
    }

    #[test]
    fn leaf_doc_renders_request_and_response_examples() {
        let info = ApiInfo {
            title: "Example API",
            version: "1.0.0",
            description: "Example description",
            servers: &[ApiServer {
                url: "https://example.com",
                description: "Mainnet",
            }],
        };

        let doc = build_leaf_doc(
            &info,
            OperationSpec {
                title: "Example API - Example Operation",
                path: "/v0/example",
                method: HttpMethod::Post,
                operation_id: "example_operation",
                summary: "Example operation",
                description: "Returns an example payload.",
                tags: &["examples"],
                request_body: Some(RequestBodySpec::Json {
                    schema: json!({"type": "object"}),
                    required: true,
                    example: None,
                    examples: vec![NamedExample {
                        name: "request",
                        summary: Some("Request example"),
                        value: json!({"id": 1}),
                    }],
                }),
                responses: vec![ResponseSpec {
                    status: "200",
                    description: "OK",
                    content: Some(ResponseContent::Json {
                        schema: json!({"type": "object"}),
                        example: Some(json!({"ok": true})),
                        examples: vec![],
                    }),
                }],
                components: json!({}),
            },
        );

        assert_eq!(doc["paths"]["/v0/example"]["post"]["operationId"], "example_operation");
        assert_eq!(
            doc["paths"]["/v0/example"]["post"]["requestBody"]["content"]["application/json"]["examples"]["request"]["value"]["id"],
            1
        );
        assert_eq!(
            doc["paths"]["/v0/example"]["post"]["responses"]["200"]["content"]["application/json"]["example"]["ok"],
            true
        );
    }

    #[test]
    fn service_doc_inlines_operations_and_extensions() {
        let info = ApiInfo {
            title: "Example API",
            version: "1.0.0",
            description: "Example description",
            servers: &[ApiServer {
                url: "https://example.com",
                description: "Mainnet",
            }],
        };

        let doc = build_service_doc(
            &info,
            vec![AggregateOperationSpec {
                slug: "example",
                title: "Example API - Example",
                path: "/v0/example/{id}",
                method: HttpMethod::Get,
                operation_id: "get_example",
                summary: "Get example",
                description: "Returns an example payload.",
                tags: &["examples"],
                parameters: vec![ParameterSpec {
                    name: "id",
                    location: ParameterLocation::Path,
                    required: true,
                    description: "Example identifier",
                    schema: json!({"type": "string"}),
                    example: Some(json!("example-id")),
                }],
                request_body: None,
                responses: vec![ResponseSpec {
                    status: "200",
                    description: "OK",
                    content: Some(ResponseContent::Json {
                        schema: json!({"$ref": "#/components/schemas/ExampleResponse"}),
                        example: None,
                        examples: vec![],
                    }),
                }],
            }],
            json!({
                "ExampleResponse": {
                    "type": "object"
                }
            }),
        );

        assert_eq!(
            doc["paths"]["/v0/example/{id}"]["get"]["x-fastnear-slug"],
            "example"
        );
        assert_eq!(
            doc["paths"]["/v0/example/{id}"]["get"]["x-fastnear-title"],
            "Example API - Example"
        );
        assert_eq!(
            doc["paths"]["/v0/example/{id}"]["get"]["parameters"][0]["in"],
            "path"
        );
        assert_eq!(
            doc["components"]["schemas"]["ExampleResponse"]["type"],
            "object"
        );
    }
}
