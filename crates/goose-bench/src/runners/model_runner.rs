use crate::bench_config::{BenchModel, BenchRunConfig};
use crate::eval_suites::EvaluationSuite;
use crate::utilities::{await_process_exits, parallel_bench_cmd, union_hashmaps};
use std::collections::HashMap;
use std::process::Child;
use std::thread;

#[derive(Clone)]
pub struct ModelRunner {
    config: BenchRunConfig,
}

impl ModelRunner {
    pub fn from(config: String) -> anyhow::Result<ModelRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(ModelRunner { config })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let model = self.config.models.first().unwrap();
        let repeat = self.config.repeat.unwrap_or(1);

        let mut handles = vec![];

        for i in 0..repeat {
            let mut self_copy = self.clone();
            let model_clone = model.clone();
            // create thread to handle launching parallel processes to run model's evals in parallel
            let handle =
                thread::spawn(move || self_copy.run_benchmark(&model_clone, i.to_string()));
            handles.push(handle);
        }

        for handle in handles {
            match handle.join() {
                Ok(_res) => {
                    // Handle the result
                    // e.g. self.handle_summary(&res)?;
                }
                Err(e) => {
                    // Handle thread panic
                    println!("Thread panicked: {:?}", e);
                }
            }
        }

        Ok(())
    }
    fn toolshim_envs(&self) -> Vec<(String, String)> {
        // read tool-shim preference from config, set respective env vars accordingly
        let mut shim_envs: Vec<(String, String)> = Vec::new();
        if let Some(shim_opt) = &self.config.tool_shim {
            if shim_opt.use_tool_shim {
                shim_envs.push(("GOOSE_TOOLSHIM".to_string(), "true".to_string()));
                if let Some(shim_model) = &shim_opt.tool_shim_model {
                    shim_envs.push((
                        "GOOSE_TOOLSHIM_OLLAMA_MODEL".to_string(),
                        shim_model.clone(),
                    ));
                }
            }
        }
        shim_envs
    }
    fn run_benchmark(&mut self, model: &BenchModel, run_id: String) -> anyhow::Result<()> {
        // convert suites map {suite_name => [eval_selector_str] to map suite_name => [BenchEval]
        let suites = self
            .config
            .evals
            .iter()
            .map(|eval| {
                EvaluationSuite::select(vec![eval.clone().selector])
                    .iter()
                    .map(|(suite, evals)| {
                        let bench_evals = evals
                            .iter()
                            .map(|suite_eval| {
                                let mut updated_eval = eval.clone();
                                updated_eval.selector = (*suite_eval).to_string();
                                updated_eval
                            })
                            .collect::<Vec<_>>();
                        (suite.clone(), bench_evals)
                    })
                    .collect()
            })
            .collect();

        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        let envs = [
            vec![(model.clone().name, model.clone().provider)],
            self.toolshim_envs(),
        ]
        .concat();

        for (suite, evals) in union_hashmaps(suites).iter() {
            results_handles.insert((*suite).clone(), Vec::new());

            // launch single suite's evals in parallel
            for eval_selector in evals {
                self.config.run_id = Some(run_id.clone());
                self.config.evals = vec![(*eval_selector).clone()];
                let cfg = self.config.to_string()?;
                let bench_cmd = "exec-eval".to_string();
                let handle = parallel_bench_cmd(bench_cmd, cfg, envs.clone());
                results_handles.get_mut(suite).unwrap().push(handle);
            }
        }

        // await all suite's evals
        for (_, child_procs) in results_handles.iter_mut() {
            await_process_exits(child_procs);
        }

        Ok(())
    }
}
