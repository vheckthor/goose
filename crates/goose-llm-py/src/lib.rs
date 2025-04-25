use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use goose_llm::completion;
use goose::message::{Message, MessageContent};
use goose::model::ModelConfig;
use mcp_core::tool::Tool;
use serde_json;

/// Python wrapper for the CompletionResponse
#[pyclass]
#[derive(Clone)]
struct PyCompletionResponse {
    #[pyo3(get)]
    message: PyMessage,
    #[pyo3(get)]
    usage: PyProviderUsage,
    #[pyo3(get)]
    tool_approvals: Option<PyToolApprovals>,
}

#[pyclass]
#[derive(Clone)]
struct PyProviderUsage {
    #[pyo3(get)]
    input_tokens: Option<i32>,
    #[pyo3(get)]
    output_tokens: Option<i32>,
    #[pyo3(get)]
    total_tokens: Option<i32>,
}

#[pyclass]
#[derive(Clone)]
struct PyToolApprovals {
    #[pyo3(get)]
    approved: Vec<String>,
    #[pyo3(get)]
    needs_approval: Vec<String>,
    #[pyo3(get)]
    denied: Vec<String>,
}

#[pyclass]
#[derive(Clone)]
struct PyMessage {
    #[pyo3(get)]
    role: String,
    #[pyo3(get)]
    content: Vec<PyMessageContent>,
}

#[pyclass]
#[derive(Clone)]
enum PyMessageContent {
    Text { text: String },
    ToolRequest { id: String, name: String, parameters: String },
    ToolResult { id: String, output: String, is_error: bool },
}

/// Convert Python dict to serde_json::Value
fn py_dict_to_json(py: Python, dict: &Bound<'_, PyDict>) -> PyResult<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        let json_value = py_to_json(py, &value)?;
        map.insert(key_str, json_value);
    }
    Ok(serde_json::Value::Object(map))
}

/// Convert Python object to serde_json::Value
fn py_to_json(py: Python, obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if let Ok(dict) = obj.downcast::<PyDict>() {
        py_dict_to_json(py, dict)
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(py_to_json(py, &item)?);
        }
        Ok(serde_json::Value::Array(vec))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(serde_json::Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap()))
    } else {
        Ok(serde_json::Value::Null)
    }
}

/// Create a Message from role and text
#[pyfunction]
fn create_message(role: &str, text: &str) -> PyResult<PyMessage> {
    let message = match role {
        "user" => Message::user().with_text(text),
        "assistant" => Message::assistant().with_text(text),
        _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid role: {}", role)
        )),
    };

    Ok(PyMessage {
        role: format!("{:?}", message.role),
        content: message.content.iter().map(|c| match c {
            MessageContent::Text(text_content) => PyMessageContent::Text { text: text_content.text.clone() },
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().expect("tool_call failed");
                PyMessageContent::ToolRequest {
                    id:         req.id.clone(),
                    name:       call.name.clone(),
                    parameters: serde_json::to_string(&call.arguments).unwrap_or_default(),
                }
            },
            _ => PyMessageContent::Text { text: "".to_string() },
        }).collect(),
    })
}

/// Create a Tool from name, description, and input schema
#[pyfunction]
fn create_tool(py: Python, name: &str, description: &str, input_schema: &Bound<'_, PyDict>) -> PyResult<PyTool> {
    let schema_json = py_dict_to_json(py, input_schema)?;
    let tool = Tool::new(name, description, schema_json, None);
    Ok(PyTool { inner: tool })
}

#[pyclass]
#[derive(Clone)]
struct PyTool {
    inner: Tool,
}

/// Perform a completion request
#[pyfunction]
fn perform_completion(
    _py: Python,
    provider: &str,
    model_name: &str,
    system_preamble: &str,
    messages: Vec<PyMessage>,
    tools: Vec<PyTool>,
    check_tool_approval: bool,
) -> PyResult<PyCompletionResponse> {
    // Create a new runtime for the async function
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Convert PyMessage to Message
    let rust_messages: Vec<Message> = messages.iter().map(|py_msg| {
        let mut msg = match py_msg.role.as_str() {
            "user" => Message::user(),
            "assistant" => Message::assistant(),
            _ => Message::user(),
        };
        for content in &py_msg.content {
            if let PyMessageContent::Text { text } = content {
                msg = msg.with_text(text);
            }
        }
        msg
    }).collect();

    let rust_tools: Vec<Tool> = tools.iter().map(|t| t.inner.clone()).collect();
    let model_config = ModelConfig::new(model_name.to_string());

    // Run the async function in the runtime
    let result = rt.block_on(async {
        completion(
            provider,
            model_config,
            system_preamble,
            &rust_messages,
            &rust_tools,
            check_tool_approval,
        ).await
    });

    match result {
        Ok(response) => {
            let message       = &response.message;
            let usage         = &response.usage;
            let tool_approvals= &response.tool_approvals;

            let py_message = PyMessage {
                role: format!("{:?}", message.role),
                content: message.content.iter().map(|c| match c {
                    MessageContent::Text(text_content) => PyMessageContent::Text { text: text_content.text.clone() },
                    MessageContent::ToolRequest(req) => {
                        let call = req.tool_call.as_ref().expect("tool_call failed");
                        PyMessageContent::ToolRequest {
                            id:         req.id.clone(),
                            name:       call.name.clone(),
                            parameters: serde_json::to_string(&call.arguments).unwrap_or_default(),
                        }
                    },
                    _ => PyMessageContent::Text { text: "".to_string() },
                }).collect(),
            };

            let py_usage = PyProviderUsage {
                input_tokens:  usage.usage.input_tokens,
                output_tokens: usage.usage.output_tokens,
                total_tokens:  usage.usage.total_tokens,
            };

            let py_tool_approvals = tool_approvals.as_ref().map(|approvals| PyToolApprovals {
                approved:       approvals.approved.clone(),
                needs_approval: approvals.needs_approval.clone(),
                denied:         approvals.denied.clone(),
            });

            Ok(PyCompletionResponse {
                message:        py_message,
                usage:          py_usage,
                tool_approvals: py_tool_approvals,
            })
        },
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Completion error: {}", e)
        )),
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn goose_llm_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_message, m)?)?;
    m.add_function(wrap_pyfunction!(create_tool, m)?)?;
    m.add_function(wrap_pyfunction!(perform_completion, m)?)?;
    m.add_class::<PyCompletionResponse>()?;
    m.add_class::<PyProviderUsage>()?;
    m.add_class::<PyToolApprovals>()?;
    m.add_class::<PyMessage>()?;
    m.add_class::<PyMessageContent>()?;
    m.add_class::<PyTool>()?;
    Ok(())
}
