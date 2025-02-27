use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct FlightDelayAnalysis {}

impl FlightDelayAnalysis {
    pub fn new() -> Self {
        FlightDelayAnalysis {}
    }
}

#[async_trait]
impl Evaluation for FlightDelayAnalysis {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Copy dataset files from assets to current directory
        let source_dir = work_dir.path.join("assets").join("small_models").join("flight_delay_analysis");
        if source_dir.exists() {
            for entry in fs::read_dir(&source_dir)? {
                let entry = entry?;
                let source_path = entry.path();
                let file_name = source_path.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
                let target_path = PathBuf::from(file_name);
                fs::copy(&source_path, &target_path)?;
            }
        }

        // Send the task prompt
        let prompt = "You need to build a model to predict whether a flight will be delayed for more than 15 minutes. \
                     The evaluation metric is ROC AUC. Please analyze the data and build an appropriate model.".to_string();
        
        let messages = agent.prompt(prompt).await?;

        // Check if the assistant demonstrates understanding of key aspects
        // let mut understands_task = false;
        // let mut understands_metric = false;
        // let mut handles_data = false;

        // for msg in messages.iter() {
        //     if msg.role == Role::Assistant {
        //         // Check message content for understanding
        //         let content_str = msg.content.iter()
        //             .filter_map(|c| match c {
        //                 MessageContent::Text(t) => Some(t.text.as_str()),
        //                 _ => None
        //             })
        //             .collect::<Vec<&str>>()
        //             .join(" ");

        //         // Check understanding of the prediction task
        //         if content_str.contains("15 minute") && content_str.contains("delay") {
        //             understands_task = true;
        //         }

        //         // Check understanding of the evaluation metric
        //         if content_str.to_lowercase().contains("roc") && content_str.to_lowercase().contains("auc") {
        //             understands_metric = true;
        //         }

        //         // Check for data handling
        //         for content in &msg.content {
        //             if let MessageContent::ToolRequest(tool_req) = content {
        //                 if let Ok(tool_call) = tool_req.tool_call.as_ref() {
        //                     if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
        //                         if tool_call.name == "developer__shell" {
        //                             if let Some(cmd) = args.get("command").and_then(Value::as_str) {
        //                                 if cmd.contains("ls") || cmd.contains("cat") || cmd.contains("head") {
        //                                     handles_data = true;
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }

        // metrics.push(("Understands prediction task".to_string(), EvaluationMetric::Boolean(understands_task)));
        // metrics.push(("Understands ROC AUC metric".to_string(), EvaluationMetric::Boolean(understands_metric)));
        // metrics.push(("Properly handles data files".to_string(), EvaluationMetric::Boolean(handles_data)));

        Ok()
    }

    fn name(&self) -> &str {
        "flight_delay_analysis"
    }
}

register_evaluation!("small_models", FlightDelayAnalysis);