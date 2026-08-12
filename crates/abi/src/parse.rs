use serde::Deserialize;
use serde_json::Value;

use crate::{AbiError, AbiEvent, AbiFormFn, AbiFormParam, AbiFormSchema, AbiWidget};

#[derive(Debug, Deserialize)]
struct RawAbiEntry {
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    inputs: Vec<RawAbiParam>,
    #[serde(default)]
    outputs: Vec<RawAbiParam>,
    #[serde(default, rename = "stateMutability")]
    state_mutability: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawAbiParam {
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    sol_type: String,
    #[serde(default)]
    components: Vec<RawAbiParam>,
}

pub fn schema_from_abi_json(json: &str) -> Result<AbiFormSchema, AbiError> {
    let value: Value = serde_json::from_str(json)?;
    schema_from_abi_value(&value)
}

pub fn schema_from_abi_value(value: &Value) -> Result<AbiFormSchema, AbiError> {
    let entries: Vec<RawAbiEntry> = match value {
        Value::Array(items) => serde_json::from_value(Value::Array(items.clone()))?,
        _ => return Err(AbiError::NotArray),
    };

    let mut schema = AbiFormSchema {
        constructor: None,
        views: Vec::new(),
        entries: Vec::new(),
        events: Vec::new(),
    };

    for entry in entries {
        match entry.entry_type.as_str() {
            "error" => {}
            "constructor" => {
                schema.constructor = Some(raw_fn_to_form_fn(
                    String::new(),
                    entry
                        .state_mutability
                        .unwrap_or_else(|| "nonpayable".into()),
                    entry.inputs,
                    entry.outputs,
                ));
            }
            "function" => {
                let name = entry.name.unwrap_or_default();
                let state_mutability = entry
                    .state_mutability
                    .unwrap_or_else(|| "nonpayable".into());
                let form_fn =
                    raw_fn_to_form_fn(name, state_mutability.clone(), entry.inputs, entry.outputs);
                match state_mutability.as_str() {
                    "view" | "pure" => schema.views.push(form_fn),
                    _ => schema.entries.push(form_fn),
                }
            }
            "event" => {
                schema.events.push(AbiEvent {
                    name: entry.name.unwrap_or_default(),
                    inputs: entry
                        .inputs
                        .into_iter()
                        .map(raw_param_to_form_param)
                        .collect(),
                });
            }
            _ => {}
        }
    }

    Ok(schema)
}

fn raw_fn_to_form_fn(
    name: String,
    state_mutability: String,
    inputs: Vec<RawAbiParam>,
    outputs: Vec<RawAbiParam>,
) -> AbiFormFn {
    AbiFormFn {
        name,
        state_mutability,
        inputs: inputs.into_iter().map(raw_param_to_form_param).collect(),
        outputs: outputs.into_iter().map(raw_param_to_form_param).collect(),
    }
}

fn raw_param_to_form_param(param: RawAbiParam) -> AbiFormParam {
    let sol_type = param.sol_type.clone();
    AbiFormParam {
        name: param.name,
        sol_type: sol_type.clone(),
        widget: sol_type_to_widget(&sol_type, !param.components.is_empty()),
    }
}

fn sol_type_to_widget(sol_type: &str, has_tuple_components: bool) -> AbiWidget {
    let sol_type = sol_type.trim();

    if has_tuple_components || is_unsupported_type(sol_type) {
        return AbiWidget::Unsupported {
            sol_type: sol_type.to_string(),
        };
    }

    match sol_type {
        "address" => AbiWidget::Address,
        "bool" => AbiWidget::Bool,
        "string" => AbiWidget::String,
        "bytes" => AbiWidget::Bytes { fixed: None },
        "uint" => AbiWidget::Uint { bits: 256 },
        "int" => AbiWidget::Int { bits: 256 },
        ty if ty.ends_with("[]") => {
            let inner_type = &ty[..ty.len() - 2];
            let inner = Box::new(sol_type_to_widget(inner_type, false));
            AbiWidget::Array { inner }
        }
        ty if ty.starts_with("bytes") => parse_fixed_bytes(ty),
        ty if ty.starts_with("uint") => parse_uint(ty),
        ty if ty.starts_with("int") => parse_int(ty),
        other => AbiWidget::Unsupported {
            sol_type: other.to_string(),
        },
    }
}

fn is_unsupported_type(sol_type: &str) -> bool {
    sol_type.starts_with('(')
        || sol_type.starts_with("tuple")
        || sol_type.starts_with("mapping")
        || sol_type.starts_with("function")
}

fn parse_fixed_bytes(sol_type: &str) -> AbiWidget {
    let suffix = &sol_type["bytes".len()..];
    if suffix.is_empty() {
        return AbiWidget::Bytes { fixed: None };
    }
    match suffix.parse::<u16>() {
        Ok(n) => AbiWidget::Bytes { fixed: Some(n) },
        Err(_) => AbiWidget::Unsupported {
            sol_type: sol_type.to_string(),
        },
    }
}

fn parse_uint(sol_type: &str) -> AbiWidget {
    let suffix = &sol_type["uint".len()..];
    if suffix.is_empty() {
        return AbiWidget::Uint { bits: 256 };
    }
    match suffix.parse::<u16>() {
        Ok(bits) => AbiWidget::Uint { bits },
        Err(_) => AbiWidget::Unsupported {
            sol_type: sol_type.to_string(),
        },
    }
}

fn parse_int(sol_type: &str) -> AbiWidget {
    let suffix = &sol_type["int".len()..];
    if suffix.is_empty() {
        return AbiWidget::Int { bits: 256 };
    }
    match suffix.parse::<u16>() {
        Ok(bits) => AbiWidget::Int { bits },
        Err(_) => AbiWidget::Unsupported {
            sol_type: sol_type.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AbiFormSchema, AbiWidget};

    fn fn_names(fns: &[AbiFormFn]) -> Vec<&str> {
        fns.iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn parse_rwa_share_registry_fixture() {
        let json = include_str!("../tests/fixtures/rwa_share_registry.abi.json");
        let schema = schema_from_abi_json(json).expect("fixture parses");

        let ctor = schema.constructor.expect("constructor present");
        assert_eq!(ctor.name, "");
        assert_eq!(ctor.inputs.len(), 3);
        assert!(ctor.inputs.iter().all(|p| p.sol_type == "uint64"));
        assert_eq!(ctor.signature(), "constructor(uint64,uint64,uint64)");
        assert!(
            ctor.inputs
                .iter()
                .all(|p| matches!(p.widget, AbiWidget::Uint { bits: 64 }))
        );

        let entry_names = fn_names(&schema.entries);
        assert!(entry_names.contains(&"setAllow"));
        assert!(entry_names.contains(&"issue"));
        assert!(entry_names.contains(&"transfer"));

        let event_names: Vec<&str> = schema.events.iter().map(|e| e.name.as_str()).collect();
        assert!(event_names.contains(&"Issued"));
        assert!(event_names.contains(&"Transferred"));
    }

    #[test]
    fn view_vs_entry_split() {
        let json = include_str!("../tests/fixtures/rwa_share_registry.abi.json");
        let schema = schema_from_abi_json(json).expect("fixture parses");

        assert!(
            schema
                .views
                .iter()
                .all(|f| { f.state_mutability == "view" || f.state_mutability == "pure" })
        );
        assert!(
            schema
                .entries
                .iter()
                .all(|f| { f.state_mutability != "view" && f.state_mutability != "pure" })
        );

        let view_names = fn_names(&schema.views);
        assert!(view_names.contains(&"owner"));
        assert!(view_names.contains(&"totalSupply"));
        assert!(!view_names.contains(&"setAllow"));
    }

    #[test]
    fn unsupported_tuple_becomes_unsupported_widget() {
        let json = r#"[
          {
            "type": "function",
            "name": "badTuple",
            "inputs": [
              {
                "name": "data",
                "type": "tuple",
                "components": [
                  { "name": "a", "type": "uint256" },
                  { "name": "b", "type": "address" }
                ]
              }
            ],
            "outputs": [],
            "stateMutability": "view"
          }
        ]"#;

        let schema = schema_from_abi_json(json).expect("tuple ABI parses");
        let param = &schema.views[0].inputs[0];
        assert!(matches!(param.widget, AbiWidget::Unsupported { .. }));
    }

    #[test]
    fn sized_integer_arrays_map_to_array_widget() {
        let json = r#"[{"type":"function","name":"xs","inputs":[{"name":"v","type":"uint256[]"}],"outputs":[],"stateMutability":"view"}]"#;
        let schema = schema_from_abi_json(json).unwrap();
        assert!(matches!(
            schema.views[0].inputs[0].widget,
            AbiWidget::Array { .. }
        ));
    }

    #[test]
    fn empty_array_yields_empty_schema() {
        let schema = schema_from_abi_json("[]").expect("empty ABI parses");
        assert_eq!(
            schema,
            AbiFormSchema {
                constructor: None,
                views: Vec::new(),
                entries: Vec::new(),
                events: Vec::new(),
            }
        );
    }
}
